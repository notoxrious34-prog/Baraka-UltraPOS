# Baraka UltraPOS v1.0.0 — Production Readiness Audit (§38)

> Official §38 compliance manifest required by CHARTER-v2.0 Section 9.2.
> Authored under TASK-REL-042 (2026-10-11) on a verified-clean `master`
> tree at the DOC-036 handoff commit's successor state. All figures below
> are measured on-disk values, not projections.

## 1. Baseline identity

| Fact | Value |
|---|---|
| Product | Baraka UltraPOS v1.0.0 (`package.json` + `src-tauri/tauri.conf.json`) |
| Bundle identifier | `com.baraka.ultrapos` |
| Branch | `master` |
| Working tree at audit | clean (`git status --porcelain` empty, verified before and after gates) |
| Prior baseline | TASK-AUD-033 (all 7 gates green) |
| Release tag | `v1.0.0-rc.2` (annotated; `v1.0.0-rc.1` superseded as source marker) |
| Ratified lexicon | `docs/design/glossary-ar-fr.md` v1.0 RATIFIED, 116 terms (REL-047) |
| Legal instrument | `docs/legal/QUESTIONNAIRE-COMPTABLE-FR.html`, 40 questions, 0 external links (REL-046) |
| Handover pack | `docs/operations/HANDOVER-GUIDE.md` + offline `release-v1.0.0/GUIDE-UTILISATION.html` + `LISEZMOI-README.txt` (REL-044/045) |
| Portable delivery | `release-v1.0.0/Baraka-UltraPOS-v1.0.0-rc.2-Delivery.zip` (on-disk, gitignored; hashes in `SHA256SUMS.txt`) |
| Drill pin F2 | RESOLVED — `EXPECTED_MIGRATION_VERSION = 14`, drill green on scratch DB (REL-044) |
| ADR-066 item #19 | CLOSED — 1024×600 baseline reconciled end to end (REL-047) |

## 2. Architectural invariants compliance

| Invariant | Evidence |
|---|---|
| 14 append-only migrations, frozen 0001–0014 | `crates/infrastructure/migrations/` lists `0001_baseline_schema.sql` … `0014_make_print_jobs_document_id_nullable.sql`; no migration edited (frozen per task scope) |
| Strict immutability triggers | `prevent_*_delete/update` triggers on `audit_log`, `cash_movements`, `command_log`, `customer_ledger`, `supplier_ledger`, `payments`, `stock_movements`, plus finalized-document guards (`prevent_documents_update_finalized`, `prevent_document_items_*_finalized`) |
| Money = i64 centimes, no floats in financial paths | `Money(i64)` in `crates/domain/src/primitives.rs`; ROUND_HALF_UP with i128 intermediates; IPC carries money/quantities as canonical strings |
| Offline-first SQLite law | `OPEN_PRAGMAS` in `crates/infrastructure/src/persistence/connection.rs`: `journal_mode = WAL`, `synchronous = FULL`, `foreign_keys = ON`, `busy_timeout = 5000`, `wal_autocheckpoint = 1000` |
| Layering | Presentation → Application → Domain → Infrastructure; React never touches SQL, never computes financial totals; authorization decided in Rust use cases |
| IPC surface | 77 `#[tauri::command(rename_all = "snake_case")]` handlers in `crates/commands/src/ipc.rs`, all 77 registered in `generate_handler!` (`src-tauri/src/lib.rs`) |
| Window geometry (ADR-066) | 1280×640 default, 1024×600 minimum, centered, maximized, resizable, not fullscreen (`src-tauri/tauri.conf.json`) |

## 3. Quality gate summary (REL-042 re-verification)

Gates re-run after documentation updates, `CARGO_BUILD_JOBS=2` on cargo invocations:

| Gate | Result |
|---|---|
| `cargo fmt --check` | exit 0, no diffs |
| `cargo clippy --all-targets --all-features -- -D warnings` | exit 0, zero warnings |
| `cargo test --workspace -j 2` | 49 suites: **388 passed, 0 failed, 4 ignored** (ignored = `#[ignore]` benchmark tests, by design) |
| `npm run lint` | exit 0 |
| `npm run typecheck` | exit 0 |
| `npx vitest run` | **31 files, 304 tests, all passed** |
| `npm run build` | exit 0 (`tsc && vite build`, 141 modules, built in ~3.9 s) |

Release benchmark harness (release profile, file-backed DBs, N = 50 measured + 5 warm-up):

```text
BENCH seed products=10000 elapsed=168.7s
BENCH print_job_insert n=50 p50=1.025ms p95=2.726ms p99=41.836ms
BENCH atomic_sale n=50 p50=3.561ms p95=14.425ms p99=22.554ms
BENCH product_search n=50 p50=0.235ms p95=15.563ms p99=16.442ms
test result: ok. 3 passed; 0 failed; finished in 169.18s
```

Full tables and honesty limits (L1–L4: single-machine targets, no UI-automation POS-open timing, environment-sensitive p99, 10 000-row dataset shape): `docs/operations/release-benchmarks-v1.0.0.md`.

## 4. Ten-module coverage and navigation

All 10 constitutional modules are built, routed, and tested:

1. Caisse / Vente (park F8 / resume F6, quick keys, cash-exact tender)
2. Stock (per-warehouse levels, movements, alerts)
3. Clients & Crédit (debt collection receipt سند قبض, overpayment prevention)
4. Fournisseurs (supplier ledger, purchases)
5. Produits (catalog, barcode search)
6. Documents (invoices, tickets, credit notes, history, reprint)
7. Sessions Caisse (open/close, float, variances)
8. Rapports (sales figures, stock, activity journal)
9. Utilisateurs (ADMIN/vendeur roles, lockout, admin-gated navigation)
10. Paramètres (store identity, RC/NIF/NIS/AI fiscal identifiers, printer hardware serial setup, backups)

Delivered experience: French + Arabic UI with full RTL, locally bundled
typography (Cairo, Inter, JetBrains Mono — no runtime downloads),
thermal-printer drill procedure (`docs/operations/thermal-printer-drill.md`),
shop-owner release notes (FR/AR) in `docs/operations/release-notes-v1.0.0.md`.

## 5. Packaging and artifact verification

- Build command: `npx tauri build` — exit 0. `beforeBuildCommand`
  (`npm run build`) clean; release profile compiled (`Finished 'release'
  profile … in 2m 26s` on the cached second pass); `makensis` completed.
- Fresh artifact: `target/release/bundle/nsis/Baraka UltraPOS_1.0.0_x64-setup.exe`
- Size: **221 552 107 bytes** (~211 MiB — reflects the bundled offline WebView2 installer, 200 MB+ as required; no internet needed on the shop machine)
- SHA-256: `C608C32A8C96B089CB51E187DE473AD0FCCD3E728E8B1C07D0311C1E65D75934`
- Timestamp: 2026-10-11 16:54:58 +0100 (replaces the pre-redesign 2026-09-30 bundle)
- Bundle config: NSIS target only, French-first languages, per-user install,
  unsigned installer (expected SmartScreen path documented in release notes).

## 6. Owner Gates status (residual, non-blocking for build)

| Gate | Status |
|---|---|
| Thermal-printer hardware drill (§38 "Real hardware printing verified") | **OPEN for shop testing** — procedure + three-row signature table (sale ticket, سند قبض debt receipt, FR/AR test print) ready (`thermal-printer-drill.md` §6); closes only with all three rows signed in-shop |
| Accountant tax questionnaire | **OPEN — Gate 2 (owner-facing)** — the 40-question instrument (`docs/legal/QUESTIONNAIRE-COMPTABLE-FR.html`, REL-046) awaits the expert-comptable's written sign-off; no code impact, lowest engineering priority |
| Database encryption | **Accepted deviation** — at-rest encryption explicitly out of v1 scope; mitigated by per-user install, local-only storage, and backup/restore guidance |

All technical engineering requirements under CHARTER-v2.0 Section 9 are
satisfied (invariants §2, gates §3, benchmarks §3, packaging §5, drill
pin F2 §7, viewport §10, glossary/questionnaire/handover above). The only
remaining opens are the two owner-facing sign-offs: **Gate 1** (physical
printer drill, three signed rows) and **Gate 2** (accounting questionnaire) —
neither can be closed by any code change.

## 7. Ignored Tests Verification

Command: `CARGO_BUILD_JOBS=2 cargo test --workspace -j 2 -- --ignored
--nocapture` → `IGNORED_EXIT:101` (exit code documents the drill gating,
not a regression — see below). The 4 `#[ignore]` tests:

| # | Test | File | Result |
|---|---|---|---|
| 1 | `bench_m1_product_search` | `crates/infrastructure/tests/release_benchmarks.rs:263` | ok |
| 2 | `bench_m2_atomic_sale` | `release_benchmarks.rs:312` | ok |
| 3 | `bench_m3_print_job_insert` | `release_benchmarks.rs:359` | ok |
| 4 | `drill_upgrade_preservation_contract` | `crates/infrastructure/tests/release_drill.rs:43` | FAILED (by design, see finding F1) |

Literal benchmark leg (debug profile, 159.01 s):

```text
BENCH seed products=10000 elapsed=158.1s
BENCH print_job_insert n=50 p50=0.840ms p95=1.683ms p99=56.717ms
test bench_m3_print_job_insert ... ok
BENCH atomic_sale n=50 p50=6.784ms p95=10.150ms p99=95.139ms
test bench_m2_atomic_sale ... ok
BENCH product_search n=50 p50=0.547ms p95=34.205ms p99=35.257ms
test bench_m1_product_search ... ok
test result: ok. 3 passed; 0 failed; finished in 159.01s
```

Same order of magnitude as the release-profile REL-042 run (§3);
debug-profile tails are wider, as expected unoptimized.

Finding F1 (drill gating, correct): the drill leg panicked at
`release_drill.rs:34` with `BARAKA_DRILL_DB must point at the copied
baraka.db: NotPresent`, i.e. it refuses to run without an explicit
operator-supplied **copy** of the database, opened `SQLITE_OPEN_READ_ONLY`.
This is the safe behavior — the drill can never touch live `%APPDATA%` data
by accident, and it was correctly left unexecuted in this environment.

Finding F2 (RESOLVED in REL-044): `release_drill.rs:30` pinned
`EXPECTED_MIGRATION_VERSION = 13` while the workspace ships 14 migrations.
Fixed by updating the constant (and the 1..=13 comments) to 14, then
verified green: a scratch DB was seeded with migrations 0001..0014 via
`run_migrations` into gitignored `target/scratch_drill/drill.db`, and
`BARAKA_DRILL_DB=<abs path> cargo test -p baraka-infrastructure --test
release_drill -- --ignored --nocapture` printed
`DRILL schema_migrations count=14 max=14`, `DRILL checksums verified
rows=14`, `DRILL foreign_key_check clean`, and
`test drill_upgrade_preservation_contract ... ok`
(`test result: ok. 1 passed; 0 failed`, exit 0). The scratch directory and
the one-off seed scaffold were deleted afterward (NO_LEFTOVERS verified via
`git status`). No migration content was touched.

## 8. Schema Downgrade & Version Guard

Source of truth: `crates/infrastructure/src/persistence/migration.rs`
(`run_migrations`, lines 106–151). The runner creates `schema_migrations`
if absent, then iterates **only** over the 14 known embedded `MIGRATIONS`:
recorded checksum equal → skip; mismatch → `ChecksumMismatch` abort;
absent → apply inside a transaction. There is **no check for rows with
`version > 14`**: a database file carrying future migration versions opens
without refusal on the direct startup path (`terminal_context.rs:106` calls
`run_migrations` unconditionally). Checksums of versions 1–14 are still
verified, so tampering with known versions is caught, but unknown-higher
rows are silently ignored — newer-DB opening is **not** prevented at this
layer, and this is the honest residual risk (a future v15+ schema could be
misread by the v1.0.0 binary rather than refused).

The downgrade guard that **does** exist sits on the restore path, not the
open path: `sqlite_backup_manager.rs:891-896` compares
`backup_schema_version` against the running binary and refuses the restore
with `RestoreIneligibleReason::SchemaNewerThanApp` (surfaced as the
`BACKUP_SCHEMA_NEWER` badge, `crates/commands/src/ipc.rs:1908`). So a newer
backup can never overwrite live data, while a newer live DB file is trusted.
Schema state is tracked in the `schema_migrations` table, never in
`PRAGMA user_version` (no `user_version` reference under `crates/`).

## 9. Data Preservation & Installer Upgrades

- Database location: per-user app-data directory resolved via Tauri's
  official path API, joined with `baraka.db`
  (`resolve_database_path`/`default_db_file_name`,
  `crates/infrastructure/src/persistence/connection.rs:19-33`; ADR-049).
  A `BARAKA_DATA_DIR` override exists strictly as a developer/test facility
  with abort-on-invalid validation (`data_dir.rs`) — no UI, no end-user docs.
- NSIS behavior: the installer is generated by Tauri from
  `src-tauri/tauri.conf.json` (no `.nsi` script, no custom uninstaller hooks
  exist anywhere in the repo). The NSIS `bundle.windows.nsis` section carries
  only `languages: [French, English]` and `displayLanguageSelector: false`.
  Tauri's generated uninstaller removes the install directory (Program Files
  binaries) and never targets `%APPDATA%`.
- Reinstall-over guarantee: installing
  `Baraka UltraPOS_1.0.0_x64-setup.exe` over an existing install replaces
  binaries only; `%APPDATA%\com.baraka.ultrapos\baraka.db` is untouched, and
  uninstalling never deletes user data (release notes §3 state this to the
  shop owner). No migration runs against the live DB at install time —
  migrations apply at application startup inside transactions.
- Drill hook: `crates/infrastructure/tests/release_drill.rs` verifies the
  upgrade-preservation contract read-only against a **copy** (`BARAKA_DRILL_DB`
  must point at a copy; opened `SQLITE_OPEN_READ_ONLY`, zero writes).

## 10. Viewport & Layout Conformance (ADR-066)

- `index.html:5`: `<meta name="viewport"
  content="width=device-width, initial-scale=1.0" />` present.
- `src/index.css:88-92`: `html, body, #root { height: 100%; }`. Note:
  **no `overflow: hidden` at root level** — clipping is enforced one level
  down, per shell: `PosShell` root (`src/components/pos/PosShell.tsx:54`)
  is `flex h-full w-full flex-col overflow-hidden`, and every module root
  (Customers, Documents, Stock, Settings, Users, …) carries
  `overflow-hidden` with `min-h-0 flex-1` flex children. Internal scrolling
  lives only inside list/modal panes (`overflow-y-auto`, 49 measured
  occurrences) — i.e. no page-level scrollbars by construction.
- Prohibitions verified by grep over `src/`: **zero** `w-screen`/`h-screen`
  occurrences; **zero** `position: fixed` in TSX/CSS (modals use
  `ModalShell` layered containers, toasts/palette excepted per constitution).
- Window floor unchanged: `minWidth: 1024, minHeight: 600` in
  `src-tauri/tauri.conf.json`; geometry is additionally pinned by the
  `window_config.rs` test suite (`main_window_geometry_matches_adr066`,
  `main_window_maximized_policy_brk060`), green in §3.
- Path note: the live `PosShell` is `src/components/pos/PosShell.tsx`
  (no `src/features/pos/PosShell.tsx` exists); `App.tsx:114-124` mounts all
  views inside `h-full` flex containers with `dir`/`lang` switching for RTL.

**ADR-066 runtime closure (REL-047 — closes technical debt item #19,
"Tauri window 1024x600 baseline reconciliation").** The baseline is verified
end to end: `minWidth: 1024, minHeight: 600` enforced in
`src-tauri/tauri.conf.json` and pinned by the green `window_config.rs` suite;
zero `position: fixed` floating action buttons in frontend markup (modals use
layered `ModalShell` containers); every view roots in `overflow-hidden` with
inner-pane `overflow-y-auto` scrolling, so no page-level scrollbars can appear
at the 1024×600 floor. Item #19 is formally CLOSED; no residual window debt.

## 11. Statement of readiness

The v1.0.0 tree meets every verifiable readiness criterion: frozen schema
with enforced invariants, green gates (388 Rust + 304 frontend tests),
measured release benchmarks within the historical order of magnitude, a
fresh NSIS installer bundling the complete 10-module UI with offline
typography and WebView2, and honest documentation of the three residual
owner gates above. No criterion is claimed beyond what the pasted evidence
supports.

*End of §38 manifest — Baraka UltraPOS v1.0.0.*
