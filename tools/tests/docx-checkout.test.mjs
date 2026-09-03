import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const root = fileURLToPath(new URL("../../", import.meta.url));
const assetPath = "crates/ort-documents/src/docx";
const assets = readdirSync(join(root, assetPath)).filter((name) =>
  name.endsWith(".xml"),
);
const attributes = readFileSync(join(root, ".gitattributes"));

// Exercise real Git checkout conversion, not a regex over .gitattributes.
// Only a fresh temporary repository/index is written; no commits or network.
function checkout(t, autocrlf, policy) {
  const scratch = mkdtempSync(join(tmpdir(), "ort-docx-checkout-"));
  t.after(() => rmSync(scratch, { recursive: true, force: true }));
  const source = join(scratch, "source");
  const output = join(scratch, "checkout");
  const empty = join(scratch, "empty");
  mkdirSync(source);
  mkdirSync(output);
  mkdirSync(empty);
  const config = join(scratch, "empty-config");
  writeFileSync(config, "");
  // Ignore machine-level Git configuration/attributes and inherited repository
  // selectors so the positive and negative controls are reproducible everywhere.
  const env = Object.fromEntries(
    Object.entries(process.env).filter(
      ([key]) => !key.toUpperCase().startsWith("GIT_"),
    ),
  );
  Object.assign(env, {
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_CONFIG_GLOBAL: config,
    GIT_ATTR_NOSYSTEM: "1",
  });
  const git = (...args) =>
    execFileSync(
      "git",
      [
        "-c",
        `core.attributesFile=${config}`,
        "-c",
        `core.hooksPath=${empty}`,
        "-c",
        "core.safecrlf=false",
        ...args,
      ],
      { cwd: source, env, stdio: "pipe", timeout: 10000 },
    );
  git("init", "--quiet", `--template=${empty}`);
  const originals = new Map();
  for (const name of assets) {
    const relative = `${assetPath}/${name}`;
    const bytes = readFileSync(join(root, relative));
    assert(!bytes.includes(13), `${relative}: embedded source must use LF`);
    assert(bytes.includes(10), `${relative}: positive line-ending control`);
    originals.set(relative, bytes);
  }
  // New template files must also match the rule; unrelated text/binary files
  // must retain ordinary Git behavior rather than a repository-wide rewrite.
  originals.set(`${assetPath}/future-template.xml`, Buffer.from("<future/>\n"));
  originals.set("unprotected.xml", Buffer.from("<control/>\n"));
  originals.set("binary.bin", Buffer.from([0, 10, 13, 10, 255]));
  for (const [relative, bytes] of originals) {
    const path = join(source, relative);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, bytes);
  }
  if (policy) writeFileSync(join(source, ".gitattributes"), policy);
  git("-c", "core.autocrlf=false", "add", "--all");
  git(
    "-c",
    `core.autocrlf=${autocrlf}`,
    "-c",
    "core.eol=crlf",
    "checkout-index",
    "--all",
    `--prefix=${output.replaceAll("\\", "/")}/`,
  );
  return {
    originals,
    read: (relative) => readFileSync(join(output, relative)),
  };
}

for (const mode of ["true", "false", "input"]) {
  test(`embedded DOCX XML stays byte-identical with core.autocrlf=${mode}`, (t) => {
    const { originals, read } = checkout(t, mode, attributes);
    for (const [relative, bytes] of originals) {
      const expected =
        relative === "unprotected.xml" && mode === "true"
          ? Buffer.from("<control/>\r\n")
          : bytes;
      assert.deepEqual(read(relative), expected, relative);
    }
  });
}

test("negative control reproduces CRLF drift without the attribute policy", (t) => {
  const { originals, read } = checkout(t, "true", null);
  for (const [relative, bytes] of originals) {
    if (relative === "binary.bin") {
      assert.deepEqual(read(relative), bytes);
    } else {
      assert.deepEqual(
        read(relative),
        Buffer.from(bytes.toString("utf8").replaceAll("\n", "\r\n")),
      );
      assert.notDeepEqual(
        read(relative),
        bytes,
        `${relative}: control must actually convert`,
      );
    }
  }
});
