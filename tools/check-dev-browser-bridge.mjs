// Exercise feature-gated native transport without weakening the default build's
// fail-closed tests. No installed host, browser, Keychain, or account is used.
import { execFileSync } from "node:child_process";

if (!["darwin", "win32"].includes(process.platform)) {
  console.log(
    "Development bridge tests require macOS or Windows; default transport remains unavailable here.",
  );
} else {
  const suites = [
    [
      "-p",
      "ort-ipc",
      "-p",
      "ort-native-host",
      "--features",
      "ort-ipc/dev-browser-bridge,ort-native-host/dev-browser-bridge",
    ],
  ];
  if (process.platform === "darwin")
    suites.push([
      "-p",
      "ort-desktop",
      "--features",
      "dev-browser-bridge",
      "browser_bridge::tests",
    ]);
  for (const args of suites) {
    execFileSync("cargo", ["test", "--locked", ...args], { stdio: "inherit" });
  }
}
