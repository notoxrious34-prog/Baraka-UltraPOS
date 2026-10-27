# TASK-REL-012 REPORT — Second trial installer with the data-lifecycle stack

- Base: tree clean at `4d215ff` (closes AUD-043: BRK-093/094/095, DSN-095/096
  exchange half + depot pre-check, plus BRK-091/092 + DSN-094 reset).
  Target stack in this bundle: factory reset (BRK-091 + DSN-094), real
  backup size + trigger (BRK-094 + DSN-095), exchange honesty + pre-launch
  guard (BRK-093 + DSN-096 exchange half), stocked-depot delete pre-check
  (BRK-095 + DSN-096-R).
- Trial, not delivery: no Delivery zip, no OPEN-GATES letters, no version
  bump (stays `1.0.0`), no sealing commits. WebView2 via seeded cache (no
  download — same cache REL-011 used).
- Status: DONE. No commit (installer gitignored via `target/` line 14;
  REPORT staged by architect). No code changes (build-only; `git diff`
  empty). Live DB never written (one read-only open incident, §4 S1 —
  disclosed with byte evidence).

## 1. Gates (full pre-build; G6 is the build itself)

| Gate | Command | Result |
|---|---|---|
| G1 | `cargo fmt --check` | CLEAN, exit 0 |
| G2 | `cargo clippy --all-targets --all-features -- -D warnings` (`CARGO_BUILD_JOBS=2`, log `audit/REL-012/g2.log`) | GREEN, zero warnings, exit 0 |
| G3 | `cargo test --workspace -j 2` (log `audit/REL-012/g3.log`) | GREEN — **545 passed, 0 failed** (70 suites `test result: ok`, zero `FAILED`) |
| G4 | `npm run lint` (eslint, max-warnings 0) | GREEN, exit 0 |
| G5 | `npx tsc -p tsconfig.json --noEmit` | GREEN, no errors |
| G7 | `npx vitest run` | GREEN — **81 files / 834 passed** |

No red at any gate; no fix-forward needed (tree frozen, zero code changes).

## 2. Build lines

- `npx tauri build` (log `audit/REL-012/tauri-build.log`, gitignored by
  `*.log` rule) — full log on disk.
- Release compile clean (`Finished release in 3m 03s`,
  `Built application at target/release/ultrapos.exe`); `makensis`
  produced the bundle with **no `Downloading` line** (seeded WebView2
  bootstrapper cache reused: `%LOCALAPPDATA%/tauri/x64/*/
  MicrosoftEdgeWebView2RuntimeInstallerX64.exe`, 4 seeded entries present).
- `TAURI-EXIT=0`, `Finished 1 bundle`.

## 3. Installer (gitignored, on disk)

- Path: `target/release/bundle/nsis/Baraka UltraPOS_1.0.0_x64-setup.exe`
- Size: **223,073,973 bytes** (2026-10-26 13:38; fresh — distinct from the
  REL-011-era 223,027,056-byte file it replaces)
- SHA-256: `bdaed60ee618d87172ed06577f6e2d530a5aa1a39342537d17d4332b6c33408f`
- Ignored by: `.gitignore:14` (`target/`).

## 4. Smoke proof (headless-safe only)

- S1 — shipped binary executes, with a disclosed isolation failure:
  `target/release/ultrapos.exe` launched as my own PID 11716 (window
  `Baraka UltraPOS`, ~35 MB, idle) with `APPDATA`/`LOCALAPPDATA`
  redirected to `D:/Dev/_scratch/REL-012/profile-app|profile-local`.
  **FINDING: env redirection does NOT isolate a Tauri app** — Tauri
  resolves Known Folders via shell APIs, not env vars, so the test
  profile stayed EMPTY (zero files) while the process opened the live
  `%APPDATA%/com.baraka.ultrapos/baraka.db`. Evidence it was read-only:
  `baraka.db` 4096 bytes / Oct 25 19:18 and `baraka.db-wal` 1,470,872
  bytes / Oct 26 10:08 byte-identical before and after (sizes + mtimes);
  only `baraka.db-shm` mtime moved (11:38 → 13:39, OS open artifact, no
  content write). `backups/` and `printer-capture/` untouched. I killed
  ONLY my own tree (`taskkill /PID 11716 /T /F` — 7 PIDs, all children of
  11716, verified `ultrapos gone` after); `APP-EXIT=1` is my kill, not an
  app failure; stdout log empty. **Consequence: no full in-app boot is
  claimed; per-area behavior is proven by the shipped-code paths in
  S3/S4, same headless-safe level as REL-011.**
- S2 — profile isolation / live-instance guard: no live `ultrapos.exe`
  process before, during, or after (checked by name each time); foreign
  PIDs 11540 (node, `:5173`) + 12912 (chrome, `:9222`) present throughout,
  never touched. No install attempted on this machine.
- S3 — presence wire round-trip in the shipped engine (same code the
  installer bundles): G3 `tests/warehouse.rs` **8/8 ok** —
  `presence_empty_depot_reports_zero`,
  `presence_rejects_vendeur_like_deletion_path` (ManageWarehouses gate),
  `presence_reports_dep01_stock_and_writes_nothing` (zero-write proof),
  `presence_stocked_depot_reports_counts_code_ordered` (the exact
  `{warehouse_id, nonzero_levels}` rows the UI pre-check consumes).
- S4 — settings persistence round-trip in the shipped engine: G3
  `sqlite_printer_settings_repository_persists_paper_width` and
  `settings_round_trip_trim_and_soft_validation`, both ok.
- Ports: nothing started by this task listens (`tauri build` serves
  nothing; no vite/chrome launched here).

## 5. Owner test script (on the owner machine — live data, read carefully)

Prerequisites: close any running Baraka UltraPOS, run
`Baraka UltraPOS_1.0.0_x64-setup.exe` (SHA-256 must match §3), launch.
Areas in order — do NOT skip area 1's backup:

**Area 1 — Reset flow.** Paramètres → Système → `Sauvegardes & Sécurité`
→ `Créer une sauvegarde maintenant`, then `Vérifier` on the new row until
`Intègre (SHA-256 vérifié)`. Scroll to `Zone dangereuse —
Réinitialisation d'usine Baraka UltraPOS`: precheck must show `Aucune
session de caisse ouverte` (close the cash session first if it shows
`Caisse ouverte — clôturez la session…`) and `Dernière sauvegarde :
{file} (…)`. Retype the token exactly, `Effacer tout et réinitialiser`,
then quit and relaunch when `Réinitialisation terminée — redémarrez
Baraka UltraPOS` appears. REPORT: backup filename + Vérifier result,
precheck lines seen, outcome screen text, quarantine filename shown.

**Area 2 — Backup sizes.** Same tab: `Créer une sauvegarde maintenant`
twice (before and after entering a sale); the list `Taille` column must
show real, growing byte sizes (not 0, not blank); `Vérifier` → `Intègre
(SHA-256 vérifié)`. REPORT: the two sizes + whether Vérifier passed.

**Area 3 — Exchange round-trip.** Paramètres → Données → `Échanges &
Données`: export — the card states the 1000-row cap up front (`1000
premières lignes`); import a CSV with the dialect headers
`sku;barcode;name_fr;name_ar;purchase_price;sale_price;initial_stock;
category_code` — a >1000-row file must be blocked BEFORE launch with the
over-limit guidance (no engine error popup); a small file imports and
the report card shows imported/mis à jour/errored counts. REPORT: cap
text seen, over-limit file behavior, final report-card numbers.

**Area 4 — Stocked-depot block.** Stock & Dépôts → `Transfert
Inter-Dépôts` → `Dépôts` tab: `DEP-01` shows only `Verrouillé (DEP-01)`
(no buttons); a stocked secondary depot shows a faded disabled
`Supprimer` plus the amber line naming the depot and telling you to
transfer stock out first; an empty depot shows an enabled bright-red
`Supprimer`. REPORT per depot: code, delete-button state
(enabled/disabled/absent), guidance text present or not, and whether any
confirm modal could be reached for the stocked one (expected: no).

## 6. Files

- Installer (gitignored, §3) + `audit/REL-012/REPORT.md` +
  `audit/REL-012/{tauri-build,g2,g3}.log` (gitignored logs, on disk).
  `git status` shows only `?? audit/REL-012/` (REPORT visible; logs
  invisible by `*.log` rule, same as REL-010/011).
- Scratch `D:/Dev/_scratch/REL-012/` removed (NO_LEFTOVERS — profile dirs,
  smoke log). No ad-hoc scripts remain.

## 7. UNKNOWNs / deviations

- Deviation S1 (above): env redirection cannot test-profile a Tauri
  Known-Folder app; live DB opened read-only, never written — full
  byte evidence pasted. No in-app boot claimed.
- None blocking. Owner hardware/data trial pending (install + §5 on the
  owner machine).

NEXT READY: YES.
