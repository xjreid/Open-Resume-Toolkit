// Local, per-user registration for the explicit macOS development channel.
import { createHash, createPublicKey } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  renameSync,
  unlinkSync,
  writeFileSync,
  chmodSync,
} from "node:fs";
import { homedir } from "node:os";
import { resolve, join } from "node:path";
import { spawnSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
const args = process.argv.slice(2);
const command = args.shift();
function option(name) {
  const index = args.indexOf(name);
  if (index < 0) return undefined;
  const value = args[index + 1];
  if (!value || value.startsWith("--"))
    throw new Error(`Missing ${name} value`);
  args.splice(index, 2);
  return value;
}
const publicFile = option("--public-key-file");
const suppliedId = option("--extension-id");
if (args.length) throw new Error(`Unknown arguments: ${args.join(" ")}`);
const keyFile = join(root, "apps/extension/manifest/chrome-dev-key.json");
const key = publicFile
  ? readFileSync(resolve(publicFile), "utf8").replace(
      /-----[^\n]+-----|\s/g,
      "",
    )
  : JSON.parse(readFileSync(keyFile, "utf8")).key;
const der = Buffer.from(key, "base64");
createPublicKey({ key: der, type: "spki", format: "der" });
const keyId = [...createHash("sha256").update(der).digest().subarray(0, 16)]
  .map((byte) => String.fromCharCode(97 + (byte >> 4), 97 + (byte & 15)))
  .join("");
const id = suppliedId ?? keyId;
if (!/^[a-p]{32}$/.test(id))
  throw new Error("Extension ID must be 32 lowercase Chrome ID letters.");
if (command === "configure-key") {
  if (!publicFile || !suppliedId || id !== keyId)
    throw new Error(
      "Provide the dashboard PUBLIC key and matching extension ID.",
    );
  const prior = JSON.parse(readFileSync(keyFile, "utf8"));
  const previousVersion = prior.version ?? "0.1.0";
  const parts = previousVersion.split(".").map(Number);
  if (prior.key !== key) parts[parts.length - 1] += 1;
  writeFileSync(
    keyFile,
    JSON.stringify({ key, version: parts.join(".") }, null, 2) + "\n",
  );
  console.log(
    `Development public key configured for ${id}. Rebuild the development ZIP and reinstall the host.`,
  );
} else if (command === "id") {
  console.log(keyId);
} else if (["install", "uninstall"].includes(command)) {
  if (process.platform !== "darwin")
    throw new Error("Development native bridge currently supports macOS only.");
  if (id !== keyId)
    throw new Error(
      "Configure the matching dashboard public key before installing this ID.",
    );
  const directory = join(
    homedir(),
    "Library/Application Support/com.openresumetoolkit.dev/browser-bridge-dev",
  );
  const binary = join(directory, "ort-native-host-dev");
  const registration = join(directory, "registration.json");
  const hosts = join(
    homedir(),
    "Library/Application Support/Google/Chrome/NativeMessagingHosts",
  );
  const manifestPath = join(hosts, "com.openresumetoolkit.dev.json");
  function regular(path) {
    if (
      existsSync(path) &&
      (!lstatSync(path).isFile() || lstatSync(path).uid !== process.getuid())
    )
      throw new Error(`Refusing unsafe development file: ${path}`);
  }
  for (const path of [binary, registration, manifestPath]) regular(path);
  if (existsSync(manifestPath)) {
    const prior = JSON.parse(readFileSync(manifestPath, "utf8"));
    if (prior.name !== "com.openresumetoolkit.dev" || prior.path !== binary)
      throw new Error(
        "Existing development manifest belongs to a different installation.",
      );
  }
  if (command === "uninstall") {
    for (const path of [manifestPath, registration, binary])
      if (existsSync(path)) unlinkSync(path);
    console.log(
      "Development host registration removed. Disable the connection in ORT if it is still running.",
    );
  } else {
    const environment = { ...process.env, ORT_DEV_CHROME_EXTENSION_ID: id };
    delete environment.ORT_CHROME_EXTENSION_ID;
    delete environment.ORT_DEV_EDGE_EXTENSION_ID;
    const build = spawnSync(
      "cargo",
      [
        "build",
        "--locked",
        "--offline",
        "-p",
        "ort-native-host",
        "--features",
        "dev-browser-bridge",
      ],
      { cwd: root, env: environment, stdio: "inherit" },
    );
    if (build.error || build.status !== 0)
      throw new Error("Development native host build failed.");
    mkdirSync(directory, { recursive: true, mode: 0o700 });
    mkdirSync(hosts, { recursive: true });
    for (const path of [directory, hosts])
      if (
        !lstatSync(path).isDirectory() ||
        lstatSync(path).uid !== process.getuid()
      )
        throw new Error("Refusing unsafe registration directory.");
    chmodSync(directory, 0o700);
    function atomic(path, contents, mode) {
      const temporary = `${path}.${process.pid}.tmp`;
      writeFileSync(temporary, contents, { flag: "wx", mode });
      renameSync(temporary, path);
    }
    const staged = `${binary}.${process.pid}.tmp`;
    copyFileSync(join(root, "target/debug/ort-native-host"), staged, 1);
    chmodSync(staged, 0o700);
    renameSync(staged, binary);
    atomic(registration, JSON.stringify({ extensionId: id }) + "\n", 0o600);
    atomic(
      manifestPath,
      JSON.stringify(
        {
          name: "com.openresumetoolkit.dev",
          description: "Open Resume Toolkit opt-in development bridge",
          path: binary,
          type: "stdio",
          allowed_origins: [`chrome-extension://${id}/`],
        },
        null,
        2,
      ) + "\n",
      0o600,
    );
    console.log(
      `Registered development host for ${id}. Open or restart the dev app; the connection enables automatically on launch.`,
    );
  }
} else
  throw new Error(
    "Usage: node tools/dev-browser-bridge.mjs id | install | uninstall | configure-key --public-key-file PATH --extension-id ID",
  );
