// Only fixed synthetic lifecycle cases. Never authorizes production imports.
export const lifecycleCases = [
  "normal",
  "cancel",
  "timeout",
  "stdout-flood",
  "stderr-flood",
  "nonzero-exit",
  "malformed-output",
  "result-without-exit",
  "eofs-without-exit",
];
const booleans = [
  "readyObserved",
  "childReaped",
  "killSent",
  "stdoutEof",
  "stderrEof",
  "accepted",
];
const integers = [
  "schemaVersion",
  "case",
  "reason",
  "stdoutBytes",
  "stderrBytes",
  "elapsedMs",
  "exitCode",
  "signal",
];
const expectedReasons = [0, 1, 2, 3, 3, 4, 5, 2, 2];
const readyBytes = Buffer.byteLength("ORT_READY_V1\n");
const resultBytes = Buffer.byteLength("ORT_READY_V1\nORT_RESULT_V1\n");

export function interpretLifecycle(measurements) {
  if (
    !Array.isArray(measurements) ||
    measurements.length !== lifecycleCases.length
  ) {
    throw new Error("Missing lifecycle case evidence.");
  }
  for (const [index, result] of measurements.entries()) {
    if (
      !result ||
      typeof result !== "object" ||
      Array.isArray(result) ||
      Object.keys(result).length !== booleans.length + integers.length ||
      [...booleans, ...integers].some((key) => !Object.hasOwn(result, key)) ||
      booleans.some((key) => typeof result[key] !== "boolean") ||
      integers.some((key) => !Number.isSafeInteger(result[key]))
    ) {
      throw new Error("Invalid lifecycle report shape or types.");
    }
    if (
      result.schemaVersion !== 1 ||
      result.case !== index ||
      result.reason !== expectedReasons[index] ||
      !result.readyObserved ||
      !result.childReaped ||
      !result.stdoutEof ||
      !result.stderrEof ||
      result.elapsedMs < 0 ||
      result.elapsedMs >= 4000 ||
      result.stdoutBytes < readyBytes ||
      result.stdoutBytes > 4097 ||
      result.stderrBytes < 0 ||
      result.stderrBytes > 4097 ||
      result.accepted !== (index === 0)
    ) {
      throw new Error(`Lifecycle case failed: ${lifecycleCases[index]}.`);
    }
    const forced = (index >= 1 && index <= 4) || index >= 7;
    if (
      result.killSent !== forced ||
      result.signal !== (forced ? 9 : 0) ||
      result.exitCode !== (forced ? -1 : index === 5 ? 65 : 0)
    ) {
      throw new Error(
        "OS termination/reaping evidence did not match the case.",
      );
    }
    if (
      ([2, 7, 8].includes(index) && result.elapsedMs < 1000) ||
      (index === 3 && result.stdoutBytes !== 4097) ||
      (index === 4 && result.stderrBytes !== 4097) ||
      (index !== 4 && result.stderrBytes !== 0) ||
      ([0, 5, 7, 8].includes(index) && result.stdoutBytes !== resultBytes) ||
      ([1, 2, 4].includes(index) && result.stdoutBytes !== readyBytes) ||
      (index === 6 && result.stdoutBytes !== readyBytes + 8)
    ) {
      throw new Error(
        "Byte-bound or deadline evidence did not match the case.",
      );
    }
  }
  return {
    normalCompletionPassed: true,
    cancellationAndTimeoutKillReapPassed: true,
    boundedStdoutAndStderrPassed: true,
    nonzeroAndMalformedOutputRejected: true,
    resultAndEofsWithoutExitRejected: true,
    fullContainmentProven: false,
    importEnabled: false,
    untested: [
      "supervisor-crash-and-parent-death-cleanup",
      "broker-mediated-descendants-and-full-tree-termination",
      "memory-cpu-thread-and-mach-port-ceilings",
      "inherited-child-filesystem-network-credential-and-ipc-authority",
      "hostile-code-launch-and-cleanup-fault-injection",
      "production-transport-integration-and-supported-os-release-matrix",
    ],
  };
}
