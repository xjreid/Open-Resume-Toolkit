import { existsSync } from "node:fs";
import { dirname, relative, resolve } from "node:path";

// Only repository destinations are checked. External sources remain subject to
// release-time verification; a local link check cannot certify their contents.
export function localLinks(markdown) {
  const prose = markdown
    .replace(/^(`{3,}|~{3,})[^\n]*\n[\s\S]*?^\1[^\n]*$/gm, "")
    .replace(/(`+)[\s\S]*?\1/g, "");
  const targets = [];
  const target = String.raw`(<[^>\n]+>|(?:\\.|[^\s()\\]|\([^()\n]*\))+)`;
  for (const pattern of [
    new RegExp(String.raw`!?\[[^\]\n]*\]\(\s*${target}`, "g"),
    new RegExp(String.raw`^\s{0,3}\[[^\]\n]+\]:\s*${target}`, "gm"),
  ]) {
    for (const match of prose.matchAll(pattern)) {
      const value = match[1].replace(/^<|>$/g, "").replace(/\\([() ])/g, "$1");
      if (!/^(?:[a-z][a-z\d+.-]*:|\/\/|#)/i.test(value)) targets.push(value);
    }
  }
  return targets;
}

export function missingLocalLinks(root, file, markdown) {
  return localLinks(markdown).flatMap((target) => {
    let path;
    try {
      path = decodeURIComponent(target.split(/[?#]/, 1)[0]);
    } catch {
      return [target];
    }
    if (!path) return [];
    const absolute = path.startsWith("/")
      ? resolve(root, "." + path)
      : resolve(dirname(file), path);
    const location = relative(root, absolute);
    return location === ".." ||
      location.startsWith("../") ||
      location.startsWith("..\\") ||
      !existsSync(absolute)
      ? [target]
      : [];
  });
}
