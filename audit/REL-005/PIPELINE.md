# TASK-REL-005 Phase B — rc.11 pipeline baseline (read-only audit)

## 1. Tauri config (`src-tauri/tauri.conf.json`, values read 2026-10-21)
- `productName`: "Baraka UltraPOS", `version`: "1.0.0", `identifier`: "com.baraka.ultrapos"
- Main window: 1280×640, `minWidth` 1024, `minHeight` 600
- `bundle.active`: true, `targets`: ["nsis"]; icons: 32x32, 128x128(+@2x), icns, ico
- Windows: `webviewInstallMode` offlineInstaller silent; NSIS languages French+English, no language selector

## 2. Version-string consistency
| source | version |
|---|---|
| `package.json` | 1.0.0 |
| root `Cargo.toml` `[workspace.package]` | 1.0.0 (edition 2021) |
| `src-tauri/Cargo.toml` | `version.workspace = true` (inherits) |
| `src-tauri/tauri.conf.json` | 1.0.0 |
Verdict: CONSISTENT (single bump point is workspace + tauri.conf + package.json).

## 3. Last provenance (rc.10, `release-v1.0.0/`)
- `Baraka UltraPOS_1.0.0_x64-setup.exe` (~222 MB) + `Baraka-UltraPOS-v1.0.0-rc.10-Delivery.zip` (~222 MB); zips also kept for rc.2/4/5/6/7/8/9
- `SHA256SUMS.txt` (2 entries), per-rc sums for rc.5/6; release notes for rc.7/8/9; `LISEZMOI-README.txt`, `GUIDE-UTILISATION.html`, `QUESTIONNAIRE-COMPTABLE-FR.html`
- Implies rc.11 delivery shape: setup exe + delivery zip + SHA256SUMS + release notes.

## 4. Build-from-clean procedure (draft for rc.11)
1. `git status` clean at release HEAD; record HEAD hash.
2. Frontend: `npm run typecheck`, `npm run lint`, `npm run build` (dist/ via Vite).
3. Rust: `CARGO_BUILD_JOBS=2 cargo fmt --check`, `clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace -j 2`.
4. Bundle: `npx tauri build` (nsis target; offline webview — no network at install).
5. Artifacts → `release-v1.0.0/`: setup exe + delivery zip (exe + notes + hashes) + fresh `SHA256SUMS.txt`.
6. NO live-DB contact at any step; migrations verified by test suite only.

## 5. Installer smoke checklist (rc.11)
- [ ] Fresh machine: installs silently (FR default), app opens, first-run setup completes, blank DB migrates to latest (check schema_migrations max version).
- [ ] Upgrade: install over rc.10 with a real shop DB copy (NEVER live %APPDATA% — copy to scratch first): opens, data preserved (spot-check products/documents/backups), FTS search returns rows.
- [ ] Fresh DB: create + search + backup + restore round-trip green.
- [ ] Uninstall leaves user data dir intact (or per policy — confirm expected behavior first).
- [ ] SHA256SUMS matches posted hashes; zip contents match checklist.
