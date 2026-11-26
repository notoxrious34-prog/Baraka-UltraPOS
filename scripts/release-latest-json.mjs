// Generates the updater `latest.json` from the signed NSIS installer.
// Reads version from package.json, signature from the `.sig` file content,
// pub_date as current UTC. Upload it to the release alongside the installer.
//
// Usage: node scripts/release-latest-json.mjs [--tag v1.4.0] [--notes "..."]

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { join, resolve } from "node:path";

const args = process.argv.slice(2);
const opt = (name, fallback) => {
  const i = args.indexOf(name);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
};

const pkg = JSON.parse(readFileSync(resolve("package.json"), "utf8"));
const version = pkg.version;
const tag = opt("--tag", `v${version}`);
const notes = opt("--notes", `Baraka UltraPOS ${tag}`);

const exeName = `Baraka UltraPOS_${version}_x64-setup.exe`;
const exePath = resolve(`target/release/bundle/nsis/${exeName}`);
const sigPath = `${exePath}.sig`;
if (!existsSync(exePath)) {
  console.error(`LATEST-FAIL: installer not found: ${exePath} (run: npx tauri build)`);
  process.exit(1);
}
if (!existsSync(sigPath)) {
  console.error(`LATEST-FAIL: signature not found: ${sigPath} (signed build required)`);
  process.exit(1);
}

const signature = readFileSync(sigPath, "utf8").trim();
const assetName = opt("--asset-name", exeName);
const url = `https://github.com/notoxrious34-prog/Baraka-UltraPOS/releases/download/${tag}/${encodeURIComponent(assetName)}`;

const latest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: { "windows-x86_64": { signature, url } },
};

const outPath = resolve(join("target/release/bundle", "latest.json"));
writeFileSync(outPath, `${JSON.stringify(latest, null, 2)}\n`);
console.log(`LATEST-OK: ${outPath}`);
