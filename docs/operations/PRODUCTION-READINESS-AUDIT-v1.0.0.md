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
| Thermal-printer hardware drill (§38 "Real hardware printing verified") | **OPEN for shop testing** — procedure + signature table ready (`thermal-printer-drill.md` §6); closes only with a signed in-shop validation row |
| Accountant tax questionnaire | **OPEN, lowest priority** — no code impact |
| Database encryption | **Accepted deviation** — at-rest encryption explicitly out of v1 scope; mitigated by per-user install, local-only storage, and backup/restore guidance |

## 7. Statement of readiness

The v1.0.0 tree meets every verifiable readiness criterion: frozen schema
with enforced invariants, green gates (388 Rust + 304 frontend tests),
measured release benchmarks within the historical order of magnitude, a
fresh NSIS installer bundling the complete 10-module UI with offline
typography and WebView2, and honest documentation of the three residual
owner gates above. No criterion is claimed beyond what the pasted evidence
supports.

*End of §38 manifest — Baraka UltraPOS v1.0.0.*
