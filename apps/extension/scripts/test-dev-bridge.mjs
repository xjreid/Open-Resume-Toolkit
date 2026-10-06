import { resolve } from "node:path";
import { spawnSync } from "node:child_process";
const root = resolve(import.meta.dirname, "../../..");
if (process.platform !== "darwin")
  throw new Error("Development bridge QA currently supports macOS only.");
const identity = spawnSync(
  "node",
  [
    resolve(root, "tools/dev-browser-bridge.mjs"),
    "id",
    ...(process.argv[2] === "store-contract" ? ["--store-test"] : []),
  ],
  { encoding: "utf8" },
);
if (identity.status !== 0)
  throw new Error("Could not resolve development extension ID.");
const env = {
  ...process.env,
  ORT_DEV_CHROME_EXTENSION_ID: identity.stdout.trim(),
  ...(process.argv[2] === "store-contract"
    ? { ORT_QA_STORE_CONTRACT: "1" }
    : {}),
};
delete env.ORT_CHROME_EXTENSION_ID;
delete env.ORT_DEV_EDGE_EXTENSION_ID;
function run(args) {
  const result = spawnSync("cargo", args, { cwd: root, env, stdio: "inherit" });
  if (result.error || result.status !== 0)
    throw new Error("Development bridge QA failed.");
}
run([
  "build",
  "--locked",
  "--offline",
  "-p",
  "ort-native-host",
  "--features",
  "dev-browser-bridge",
]);
run([
  "test",
  "--locked",
  "--offline",
  "-p",
  "ort-desktop",
  "--features",
  "dev-browser-bridge",
  "--lib",
  "development_chrome_native_roundtrip",
  "--",
  "--ignored",
  "--nocapture",
]);
