import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { basename, dirname, join } from "node:path";

export function forbiddenSecretPath(path) {
  const name = basename(path).toLowerCase();
  return (
    (/^\.env(?:\.|$)/.test(name) &&
      !/^\.env\.(example|sample|template)$/.test(name)) ||
    /^(?:id_(?:rsa|dsa|ecdsa|ed25519)|\.netrc|_netrc|\.npmrc|\.pypirc|\.git-credentials|credentials(?:\.json)?|auth\.json|service[-_]account.*\.json)$/.test(
      name,
    ) ||
    /\.(?:pem|key|p12|pfx|jks|keystore|mobileprovision|sqlite3?|db)$/.test(
      name,
    ) ||
    /(?:^|\/)(?:\.aws|\.ssh|\.gnupg|\.ort-dev-data|\.ort-test-data)(?:\/|$)/.test(
      path,
    )
  );
}

export function inspectScanDirectory(directory) {
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry);
    const metadata = lstatSync(path);
    if (metadata.isSymbolicLink())
      throw new Error(`Secret scan rejects symlinks: ${path}`);
    if (metadata.isDirectory()) inspectScanDirectory(path);
    else if (!metadata.isFile() || forbiddenSecretPath(path))
      throw new Error(`Sensitive or unsupported file: ${path}`);
  }
}

export function scanSecrets(
  binary,
  config,
  directory,
  temporary,
  history = false,
) {
  const root = dirname(config);
  const rules = readFileSync(
    join(root, "target/security-tools/gitleaks-rules.toml"),
  );
  if (
    createHash("sha256").update(rules).digest("hex") !==
    "be04d7136dacfd77f569838e3b7c02cee9a77f37658629378b68d567ada8429f"
  )
    throw new Error(
      "Scanner rules changed; run python3 tools/install-gitleaks.py.",
    );
  const version = spawnSync(binary, ["version"], { encoding: "utf8" });
  if (version.status !== 0 || version.stdout.trim() !== "8.30.1")
    throw new Error(
      "Gitleaks 8.30.1 is required. Run python3 tools/install-gitleaks.py.",
    );
  if (!history) inspectScanDirectory(directory);
  const report = join(temporary, "findings.json");
  const result = spawnSync(
    binary,
    [
      history ? "git" : "dir",
      directory,
      "--config",
      config,
      "--redact=100",
      "--no-banner",
      "--no-color",
      "--ignore-gitleaks-allow",
      "--gitleaks-ignore-path",
      temporary,
      "--max-archive-depth=3",
      "--max-decode-depth=5",
      "--report-format=json",
      "--report-path",
      report,
      ...(history ? ["--log-opts=--all --full-history"] : []),
    ],
    { cwd: root, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 },
  );
  // Only report locations/rules, never raw matching lines or scanner output.
  // Read/parse errors must fail even if the scanner returns exit code zero.
  if (
    result.error ||
    ![0, 1].includes(result.status) ||
    /\b(?:ERR|FTL)\b/.test(result.stderr || "")
  )
    throw new Error(
      "Secret scanner could not complete; refusing to pass the gate.",
    );
  const findings = JSON.parse(readFileSync(report, "utf8"));
  if (result.status !== 0 || findings.length) {
    const locations = findings.map(
      (finding) =>
        `${finding.File}:${finding.StartLine} (${finding.RuleID}${finding.Commit ? `, commit ${finding.Commit.slice(0, 12)}` : ""})`,
    );
    throw new Error(
      `Potential secrets detected; values withheld:\n${locations.join("\n")}`,
    );
  }
}
