import { readFileSync, readdirSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("native-only export authority", () => {
  it("does not grant webviews dialog, filesystem, shell or process permissions", () => {
    const root = new URL("../src-tauri/capabilities/", import.meta.url);
    for (const file of readdirSync(root)) {
      if (!file.endsWith(".json")) continue;
      const capability = JSON.parse(readFileSync(new URL(file, root), "utf8"));
      expect(capability.permissions).toEqual(["core:default"]);
      expect(capability.remote).toBeUndefined();
    }
    const config = JSON.parse(
      readFileSync(
        new URL("../src-tauri/tauri.conf.json", import.meta.url),
        "utf8",
      ),
    );
    expect(config.app.security.csp).not.toContain("unsafe-eval");
    expect(config.app.security.csp).not.toContain("unsafe-inline");
  });
});
