// Exercise feature-gated macOS transport without weakening the default build's
// fail-closed tests. No installed host, browser, Keychain, or account is used.
import { execFileSync } from "node:child_process";

if (process.platform !== "darwin") {
  console.log(
    "Development bridge tests require macOS; default transport remains unavailable here.",
  );
} else {
  for (const args of [
    [
      "-p",
      "ort-ipc",
      "-p",
      "ort-native-host",
      "--features",
      "ort-ipc/dev-browser-bridge,ort-native-host/dev-browser-bridge",
    ],
    [
      "-p",
      "ort-desktop",
      "--features",
      "dev-browser-bridge",
      "browser_bridge::tests",
    ],
  ]) {
    execFileSync("cargo", ["test", "--locked", ...args], { stdio: "inherit" });
  }
}
