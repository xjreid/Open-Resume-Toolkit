import assert from "node:assert/strict";

export const developmentId = "com.openresumetoolkit.dev";
export const productionCsp =
  "default-src 'self'; script-src 'self'; worker-src 'self'; style-src 'self'; img-src 'self' asset: data:; font-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; frame-src 'none'; form-action 'none'";

// An intentional policy change must update this reviewed allowlist and tests.
export function verifyConfiguration(config, capabilities) {
  assert.equal(config.identifier, developmentId);
  assert.equal(config.productName, "Open Resume Toolkit Dev");
  assert.equal(config.build.frontendDist, "../dist");
  assert.equal(config.build.beforeBuildCommand, "pnpm build:web");
  assert.equal(config.app.withGlobalTauri, false);
  assert.equal(config.app.security.csp, productionCsp);
  assert.deepEqual(config.app.security.capabilities, ["main", "overlay"]);
  assert.equal(
    config.app.security.dangerousDisableAssetCspModification,
    undefined,
  );
  assert.equal(config.app.security.assetProtocol, undefined);
  assert.equal(config.app.security.pattern, undefined);
  assert.equal(config.bundle.externalBin, undefined);
  assert.equal(config.bundle.resources, undefined);
  assert.equal(config.plugins, undefined);
  assert.deepEqual(
    config.app.windows.map(({ label, url }) => ({ label, url })),
    [
      { label: "main", url: "index.html" },
      { label: "overlay", url: "overlay.html" },
    ],
  );
  for (const window of config.app.windows) {
    assert.equal(window.devtools, undefined);
    assert.equal(window.additionalBrowserArgs, undefined);
  }
  assert.equal(capabilities.length, 2);
  for (const [index, capability] of capabilities.entries()) {
    const label = ["main", "overlay"][index];
    assert.equal(capability.identifier, label);
    assert.deepEqual(capability.windows, [label]);
    assert.deepEqual(capability.permissions, ["core:default"]);
    assert.equal(capability.remote, undefined);
    assert.equal(capability.webviews, undefined);
    assert.notEqual(capability.local, false);
  }
}

export function verifyLocalAssetReference(reference) {
  assert.ok(reference.length > 0, "empty asset reference");
  assert.ok(!reference.startsWith("//"), "network asset");
  assert.ok(!/^[a-z][a-z\d+.-]*:/i.test(reference), "asset scheme");
  assert.ok(!reference.includes("\\"), "ambiguous asset separator");
  assert.ok(
    !decodeURIComponent(reference).split("/").includes(".."),
    "asset traversal",
  );
}

export function verifySignature(
  details,
  entitlements,
  certificateSha256,
  expected,
) {
  assert.match(details, /Identifier=com\.openresumetoolkit\.dev(?:\s|$)/);
  assert.match(details, /\(runtime\)/, "hardened runtime is required");
  assert.doesNotMatch(details, /Signature=adhoc/);
  assert.equal(certificateSha256, expected, "unexpected signing certificate");
  assert.deepEqual(
    entitlements,
    {},
    "M0 shell requires no signing entitlements",
  );
}

export function designatedRequirement(output) {
  // codesign writes the requirement and executable path to different streams.
  // Compare the requirement itself, independent of stream order/app location.
  const lines = output
    .split(/\r?\n/)
    .filter((line) => line.startsWith("designated => "));
  assert.equal(
    lines.length,
    1,
    "exactly one designated requirement is required",
  );
  assert.ok(lines[0].length > "designated => ".length);
  return lines[0];
}
