// Public dashboard identity only. No Google account credential is requested.
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { chromeIdentity } from "./lib/chrome-extension-identity.mjs";

const args = process.argv.slice(2);
function option(name) {
  const index = args.indexOf(name);
  if (index < 0 || !args[index + 1] || args[index + 1].startsWith("--"))
    throw new Error(`Provide ${name}.`);
  const value = args[index + 1];
  args.splice(index, 2);
  return value;
}
const publicFile = option("--public-key-file");
const extensionId = option("--extension-id");
if (args.length) throw new Error(`Unknown arguments: ${args.join(" ")}`);
const identity = chromeIdentity(
  readFileSync(resolve(publicFile), "utf8"),
  extensionId,
);
const path = resolve(
  import.meta.dirname,
  "../apps/extension/manifest/chrome-store-key.json",
);
writeFileSync(path, `${JSON.stringify(identity, null, 2)}\n`);
console.log(
  `Configured Chrome Store identity ${identity.extensionId}. Build chrome:store-test for a matching unpacked copy. The upload ZIP stays on the production host.`,
);
