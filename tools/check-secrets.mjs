import { execFileSync } from "node:child_process";
import {
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { forbiddenSecretPath, scanSecrets } from "./lib/secret-scanning.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));
const git = (args) =>
  execFileSync("git", args, { cwd: root, maxBuffer: 64 * 1024 * 1024 });
const [mode = "--source", ...paths] = process.argv.slice(2);
const binary =
  process.env.GITLEAKS_BIN ||
  join(
    root,
    "target/security-tools",
    process.platform === "win32" ? "gitleaks.exe" : "gitleaks",
  );
const config = join(root, ".gitleaks.toml");
const temporary = mkdtempSync(join(tmpdir(), "ort-secret-scan-"));

try {
  if (
    !["--source", "--staged", "--history", "--artifacts"].includes(mode) ||
    (mode !== "--artifacts" && paths.length) ||
    (mode === "--artifacts" && !paths.length)
  )
    throw new Error(
      "Use check-secrets.mjs [--source|--staged|--history|--artifacts PATH...].",
    );
  if (mode === "--history") {
    if (
      git(["rev-parse", "--is-shallow-repository"]).toString().trim() !==
      "false"
    )
      throw new Error(
        "Full-history scanning requires a complete clone (fetch-depth: 0).",
      );
    const historicalPaths = git([
      "log",
      "--all",
      "--format=",
      "--name-only",
      "-z",
    ])
      .toString()
      .split("\0")
      .map((path) => path.replace(/^\n+/, ""))
      .filter(Boolean);
    for (const path of historicalPaths)
      if (forbiddenSecretPath(path))
        throw new Error(`Sensitive file in Git history: ${path}`);
    scanSecrets(binary, config, root, temporary, true);
  } else if (mode === "--artifacts") {
    for (const path of paths) {
      const directory = resolve(path);
      if (
        !existsSync(directory) ||
        !lstatSync(directory).isDirectory() ||
        lstatSync(directory).isSymbolicLink()
      )
        throw new Error(`Missing or unsafe artifact directory: ${path}`);
      scanSecrets(binary, config, directory, temporary);
    }
  } else {
    const staged = mode === "--staged";
    if (staged) {
      const entries = git(["ls-files", "--stage", "-z"]).toString().split("\0");
      for (const entry of entries) {
        if (
          entry &&
          !entry.startsWith("100644 ") &&
          !entry.startsWith("100755 ")
        )
          throw new Error(
            "The staged secret scan requires regular, resolved files.",
          );
      }
    }
    const files = git(
      staged
        ? ["ls-files", "--cached", "-z"]
        : ["ls-files", "--cached", "--others", "--exclude-standard", "-z"],
    )
      .toString()
      .split("\0")
      .filter(Boolean);
    const snapshot = join(temporary, "source");
    mkdirSync(snapshot);
    for (const path of new Set(files)) {
      if (forbiddenSecretPath(path))
        throw new Error(`Sensitive repository file: ${path}`);
      const source = join(root, path);
      if (!staged && !existsSync(source)) continue;
      if (!staged && !lstatSync(source).isFile())
        throw new Error(`Non-regular source file: ${path}`);
      const content = staged ? git(["show", `:${path}`]) : readFileSync(source);
      const destination = join(snapshot, path);
      mkdirSync(dirname(destination), { recursive: true });
      writeFileSync(destination, content);
    }
    scanSecrets(binary, config, snapshot, temporary);
  }
  console.log(`Secret scan passed (${mode.slice(2)}; Gitleaks 8.30.1).`);
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
