const operators = new Set(["AND", "OR", "WITH"]);

function tokenize(expression) {
  const normalized = expression.replaceAll(/\s*\/\s*/g, " OR ").trim();
  const tokens = normalized.match(/\(|\)|[A-Za-z0-9.+-]+/g) ?? [];
  if (tokens.join("") !== normalized.replaceAll(/\s+/g, "")) {
    throw new Error(`unsupported SPDX syntax: ${expression}`);
  }
  return tokens;
}

/**
 * Return whether an SPDX expression has at least one policy-compatible choice.
 * OR selects a compatible branch; AND requires every term in that branch.
 */
export function licenseExpressionAllowed(
  expression,
  allowedLicenses,
  allowedExceptions,
) {
  if (typeof expression !== "string" || expression.trim() === "") return false;
  const tokens = tokenize(expression);
  let cursor = 0;

  function primary() {
    const token = tokens[cursor++];
    if (token === "(") {
      const value = orExpression();
      if (tokens[cursor++] !== ")")
        throw new Error("missing closing parenthesis");
      return value;
    }
    if (
      !token ||
      token === ")" ||
      operators.has(token) ||
      !/^[A-Za-z0-9][A-Za-z0-9.+-]*$/.test(token)
    ) {
      throw new Error(`expected license identifier, found ${token ?? "end"}`);
    }
    return allowedLicenses.has(token);
  }

  function withExpression() {
    let value = primary();
    if (tokens[cursor] === "WITH") {
      cursor += 1;
      const exception = tokens[cursor++];
      if (!exception || operators.has(exception) || exception === ")") {
        throw new Error("expected SPDX exception after WITH");
      }
      value = value && allowedExceptions.has(exception);
    }
    return value;
  }

  function andExpression() {
    let value = withExpression();
    while (tokens[cursor] === "AND") {
      cursor += 1;
      value = withExpression() && value;
    }
    return value;
  }

  function orExpression() {
    let value = andExpression();
    while (tokens[cursor] === "OR") {
      cursor += 1;
      value = andExpression() || value;
    }
    return value;
  }

  const result = orExpression();
  if (cursor !== tokens.length) {
    throw new Error(`unexpected SPDX token ${tokens[cursor]}`);
  }
  return result;
}

export function pnpmPackageKeys(lockfile) {
  const packages = lockfile.match(
    /^packages:\s*$([\s\S]*?)^snapshots:\s*$/m,
  )?.[1];
  if (!packages)
    throw new Error("pnpm lockfile has no packages/snapshots sections");
  const keys = [];
  for (const line of packages.split(/\r?\n/)) {
    const match = line.match(/^  (['"]?)(\S.*?)\1:\s*$/);
    if (match) keys.push(match[2]);
  }
  if (keys.length === 0) throw new Error("pnpm lockfile contains no packages");
  return keys;
}

export function splitPackageKey(key) {
  const separator = key.lastIndexOf("@");
  if (separator <= 0 || separator === key.length - 1) {
    throw new Error(`unsupported pnpm package key: ${key}`);
  }
  return { name: key.slice(0, separator), version: key.slice(separator + 1) };
}

export function platformFamilyLicense(key, families) {
  const { name, version } = splitPackageKey(key);
  const matches = families.filter(
    (family) => name.startsWith(family.prefix) && version === family.version,
  );
  if (matches.length > 1) {
    throw new Error(`multiple platform-family policies match ${key}`);
  }
  return matches[0]?.license ?? null;
}

export function licenseExceptionAllowed(exception, license, today) {
  return Boolean(
    exception?.license === license &&
      exception.reason?.length >= 40 &&
      /^\d{4}-\d{2}-\d{2}$/.test(exception.expiresOn ?? "") &&
      exception.expiresOn >= today,
  );
}

// Single-OS optional packages have no installed sibling on other OSes. Their
// reviewed metadata is bound to an exact lockfile digest and OS restriction.
export function platformPackageLicense(
  key,
  records,
  lockfile,
  platform,
  installed,
) {
  const matches = records.filter((record) => record.package === key);
  if (matches.length === 0) return null;
  if (matches.length !== 1)
    throw new Error(`duplicate platform-package policy: ${key}`);
  const record = matches[0];
  const section =
    lockfile.match(/^packages:\s*$([\s\S]*?)^snapshots:\s*$/m)?.[1] ?? "";
  const blocks = [
    ...section.matchAll(
      /^  (['"]?)(\S.*?)\1:[ \t]*\r?\n((?: {4}[^\n]*\n|\r?\n)*)/gm,
    ),
  ];
  const block = blocks.filter((match) => match[2] === key);
  if (
    block.length !== 1 ||
    !block[0][3].includes(`resolution: {integrity: ${record.integrity}}`) ||
    !block[0][3]
      .split(/\r?\n/)
      .some((line) => line.trim() === `os: [${record.os}]`)
  ) {
    throw new Error(`${key}: reviewed platform metadata differs from lockfile`);
  }
  if (installed) {
    const { name, version } = splitPackageKey(key);
    if (
      installed.name !== name ||
      installed.version !== version ||
      installed.license !== record.license ||
      JSON.stringify(installed.os) !== JSON.stringify([record.os])
    ) {
      throw new Error(
        `${key}: installed metadata differs from reviewed policy`,
      );
    }
  } else if (platform === record.os || !record.absentOn.includes(platform)) {
    throw new Error(`${key}: package must be installed on ${platform}`);
  }
  return record.license;
}
