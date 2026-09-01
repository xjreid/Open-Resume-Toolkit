import { readFileSync } from "node:fs";

const config = JSON.parse(
  readFileSync(
    new URL("../apps/desktop/src-tauri/tauri.conf.json", import.meta.url),
  ),
);

if (
  config.identifier !== "com.openresumetoolkit.dev" ||
  !config.productName.endsWith("Dev")
) {
  console.error(
    "Refusing to run: the development command must use the isolated dev identity.",
  );
  process.exit(1);
}

console.log(`Using isolated application identity: ${config.identifier}`);
