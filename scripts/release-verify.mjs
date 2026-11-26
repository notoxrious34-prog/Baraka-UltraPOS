// G8 release-artifact verifier (docs/operations/local-signing.md, step 3).
//
// Key-free: checks the Tauri bundle output for the installer, its detached
// minisign `.sig`, and `latest.json`; prints size + SHA-256 per artifact
// (the evidence chain the release notes require). Exit 0 = publishable,
// exit 1 = precise missing-artifact list (environmental, not a defect).
//
// Usage: npm run release:verify [-- <bundle-dir>]

import { createHash } from "node:crypto";
import { readdirSync, statSync, readFileSync, existsSync } from "node:fs";
import { join, resolve } from "node:path";

const candidates = process.argv[2]
  ? [resolve(process.argv[2])]
  : [resolve("target/release/bundle"), resolve("src-tauri/target/release/bundle")];
const bundleDir = candidates.find((dir) => existsSync(dir)) ?? candidates[0];
const pkgVersion = JSON.parse(readFileSync(resolve("package.json"), "utf8")).version;
const versionTag = `_${pkgVersion}_`;
const nsisDir = join(bundleDir, "nsis");

function sha256Hex(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

const failures = [];
const evidence = [];

if (!existsSync(nsisDir)) {
  failures.push(`missing nsis dir: ${nsisDir} (run: npx tauri build)`);
} else {
  const installers = readdirSync(nsisDir).filter(
    (f) => f.endsWith("-setup.exe") && f.includes(versionTag),
  );
  if (installers.length === 0) {
    failures.push(`no *-setup.exe matching version ${pkgVersion} in ${nsisDir}`);
  }
  for (const exe of installers) {
    const exePath = join(nsisDir, exe);
    const sigPath = `${exePath}.sig`;
    const size = statSync(exePath).size;
    if (!existsSync(sigPath)) {
      failures.push(`missing signature: ${exe}.sig (owner step: set TAURI_SIGNING_PRIVATE_KEY and rebuild)`);
    } else {
      evidence.push({ file: exe, bytes: size, sha256: sha256Hex(exePath) });
      evidence.push({ file: `${exe}.sig`, bytes: statSync(sigPath).size, sha256: sha256Hex(sigPath) });
    }
  }
}

const latestPath = join(bundleDir, "latest.json");
if (!existsSync(latestPath)) {
  failures.push(`missing latest.json in ${bundleDir} (upload alongside the installer)`);
} else {
  evidence.push({
    file: "latest.json",
    bytes: statSync(latestPath).size,
    sha256: sha256Hex(latestPath),
  });
}

for (const row of evidence) {
  console.log(`${row.file}  bytes=${row.bytes}  sha256=${row.sha256}`);
}

if (failures.length > 0) {
  for (const failure of failures) console.error(`G8-FAIL: ${failure}`);
  process.exit(1);
}
console.log("G8-OK: installer + .sig + latest.json present");
