// Only interprets synthetic measurements. This is NOT a product feature gate.
const booleans = ["descriptorRead", "descriptorReadOnly", "sandboxEntitlement"];
const outcomes = [
  "siblingRead",
  "siblingWrite",
  "symlinkRead",
  "loopbackConnect",
  "childCreation",
];

function exactKeys(value, keys) {
  if (
    !value ||
    typeof value !== "object" ||
    Array.isArray(value) ||
    Object.keys(value).length !== keys.length ||
    !keys.every((key) => Object.hasOwn(value, key))
  ) {
    throw new Error("Invalid synthetic probe report shape.");
  }
}

export function interpretProbe(measurement) {
  exactKeys(measurement, [
    "schemaVersion",
    "control",
    "sandboxed",
    "cooperativeDisconnectObserved",
  ]);
  if (
    measurement.schemaVersion !== 1 ||
    measurement.cooperativeDisconnectObserved !== true
  ) {
    throw new Error("Probe version or cooperative lifecycle check failed.");
  }
  for (const result of [measurement.control, measurement.sandboxed]) {
    exactKeys(result, [...booleans, ...outcomes]);
    if (
      booleans.some((key) => typeof result[key] !== "boolean") ||
      outcomes.some((key) => ![0, 1].includes(result[key]))
    ) {
      // Native enum 2 means environment/probe error, never access denial.
      throw new Error("Probe measurement is invalid or inconclusive.");
    }
    if (!result.descriptorRead || !result.descriptorReadOnly) {
      throw new Error("Read-only descriptor positive control failed.");
    }
  }
  if (
    measurement.control.sandboxEntitlement ||
    !measurement.sandboxed.sandboxEntitlement ||
    outcomes.some((key) => measurement.control[key] !== 0)
  ) {
    throw new Error(
      "Unsandboxed positive control or helper entitlement check failed.",
    );
  }
  const observed = measurement.sandboxed;
  return {
    descriptorBoundaryPassed: true,
    filesystemIsolationPassed: [
      "siblingRead",
      "siblingWrite",
      "symlinkRead",
    ].every((key) => observed[key] === 1),
    loopbackConnectDenied: observed.loopbackConnect === 1,
    childCreationDenied: observed.childCreation === 1,
    cooperativeDisconnectObserved: true,
    // Even every measured denial leaves resource/credential/broker/forced-kill
    // tests missing. Never equate this subset or a green probe job to full proof.
    fullContainmentProven: false,
    importEnabled: false,
    untested: [
      "forced-process-tree-termination-and-parent-death",
      "memory-cpu-handle-ceilings",
      "credential-and-broker-access",
      "udp-dns-and-non-loopback-network",
      "filesystem-outside-the-synthetic-fixtures",
      "hostile-code-and-crash-cleanup",
      "signed-release-and-supported-os-matrix",
    ],
  };
}
