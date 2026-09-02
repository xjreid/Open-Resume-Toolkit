// Only interprets synthetic measurements. This is NOT a product feature gate.
const booleans = ["descriptorRead", "descriptorReadOnly", "sandboxEntitlement"];
const outcomes = [
  "siblingRead",
  "siblingWrite",
  "symlinkRead",
  "loopbackConnect",
  "childCreation",
  "childFork",
];
const expectedLimits = {
  nprocSoft: 0,
  nprocHard: 0,
  nofileSoft: 64,
  nofileHard: 64,
  coreSoft: 0,
  coreHard: 0,
};
const limitChecks = [
  "raiseDenied",
  "descriptorCeilingDenied",
  "descriptorRecovery",
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
    "hardened",
    "hardLimits",
    "parentUnaffected",
    "cooperativeDisconnectObserved",
  ]);
  if (
    measurement.schemaVersion !== 2 ||
    measurement.cooperativeDisconnectObserved !== true ||
    measurement.parentUnaffected !== true
  ) {
    throw new Error("Probe version or cooperative lifecycle check failed.");
  }
  for (const result of [
    measurement.control,
    measurement.sandboxed,
    measurement.hardened,
  ]) {
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
    !measurement.hardened.sandboxEntitlement ||
    outcomes.some((key) => measurement.control[key] !== 0)
  ) {
    throw new Error(
      "Unsandboxed positive control or helper entitlement check failed.",
    );
  }
  // Require same-helper positive controls before interpreting the EAGAIN-based
  // hard-limit denials. A baseline already unable to fork is not proof of NPROC.
  if (
    measurement.sandboxed.childCreation !== 0 ||
    measurement.sandboxed.childFork !== 0
  ) {
    throw new Error("Same-helper child-creation positive control failed.");
  }
  exactKeys(measurement.hardLimits, [
    ...Object.keys(expectedLimits),
    ...limitChecks,
  ]);
  if (
    Object.entries(expectedLimits).some(
      ([key, value]) => measurement.hardLimits[key] !== value,
    ) ||
    limitChecks.some((key) => typeof measurement.hardLimits[key] !== "boolean")
  ) {
    throw new Error("Unexpected or invalid hard-limit policy.");
  }
  const observed = measurement.hardened;
  return {
    descriptorBoundaryPassed: true,
    filesystemIsolationPassed: [
      "siblingRead",
      "siblingWrite",
      "symlinkRead",
    ].every((key) => observed[key] === 1 && measurement.sandboxed[key] === 1),
    loopbackConnectDenied:
      observed.loopbackConnect === 1 &&
      measurement.sandboxed.loopbackConnect === 1,
    baselineChildCreationDenied: false,
    directChildCreationDenied:
      observed.childCreation === 1 && observed.childFork === 1,
    hardLimitRaiseDenied: measurement.hardLimits.raiseDenied,
    descriptorCeilingEnforced:
      measurement.hardLimits.descriptorCeilingDenied &&
      measurement.hardLimits.descriptorRecovery,
    coreDumpLimitZero: true,
    parentUnaffected: true,
    cooperativeDisconnectObserved: true,
    // Even every measured denial leaves resource/credential/broker/forced-kill
    // tests missing. Never equate this subset or a green probe job to full proof.
    fullContainmentProven: false,
    importEnabled: false,
    untested: [
      "forced-process-tree-termination-and-parent-death",
      "memory-cpu-thread-and-mach-port-ceilings",
      "broker-mediated-process-creation-and-exec-replacement",
      "hostile-code-core-dump-and-diagnostic-artifact-verification",
      "credential-and-broker-access",
      "udp-dns-and-non-loopback-network",
      "filesystem-outside-the-synthetic-fixtures",
      "hostile-code-and-crash-cleanup",
      "signed-release-and-supported-os-matrix",
    ],
  };
}
