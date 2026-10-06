import { createHash, createPublicKey } from "node:crypto";
import { readFileSync } from "node:fs";

export function chromeIdentity(publicKey, expectedId) {
  if (typeof publicKey !== "string" || /PRIVATE KEY/.test(publicKey))
    throw new Error(
      "Use the Chrome dashboard PUBLIC key, never a private key.",
    );
  const key = publicKey.replace(
    /-----BEGIN PUBLIC KEY-----|-----END PUBLIC KEY-----|\s/g,
    "",
  );
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(key))
    throw new Error("Invalid public key.");
  const der = Buffer.from(key, "base64");
  if (der.toString("base64") !== key)
    throw new Error("Invalid public key encoding.");
  const parsed = createPublicKey({ key: der, type: "spki", format: "der" });
  if (
    parsed.asymmetricKeyType !== "rsa" ||
    parsed.asymmetricKeyDetails.modulusLength < 2048
  )
    throw new Error("Expected Chrome's RSA public key (at least 2048 bits).");
  const extensionId = [
    ...createHash("sha256").update(der).digest().subarray(0, 16),
  ]
    .map((byte) => String.fromCharCode(97 + (byte >> 4), 97 + (byte & 15)))
    .join("");
  if (expectedId !== undefined && expectedId !== extensionId)
    throw new Error(
      "The dashboard public key does not match the extension ID.",
    );
  return { key, extensionId };
}

export function readChromeIdentity(path) {
  const value = JSON.parse(readFileSync(path, "utf8"));
  if (!value.extensionId || !value.key)
    throw new Error(
      "Upload the production ZIP as a dashboard draft, then configure its public key and ID.",
    );
  return chromeIdentity(value.key, value.extensionId);
}
