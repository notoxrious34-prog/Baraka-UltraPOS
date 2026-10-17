# Release Notes — Baraka UltraPOS v1.0.0-rc.10

- Release Candidate: **v1.0.0-rc.10**
- Date: 2026-10-15
- Git tag: `v1.0.0-rc.10` (annotated; `v1.0.0` remains prohibited by Charter §3.1)
- Baseline: commits `1b3ec23` (DSN-050 warehouse selection + instant search),
  `a9e66bc` (BRK-074 dual-track deletion engine + configurable timbre),
  `e81d80c` (DSN-051 safe-deletion UI + fiscal toggle),
  `591dc0d` (DSN-052 starter-pack lifecycle)

## Verification metrics

- Rust workspace: **462 passed / 0 failed** (`cargo fmt --check` clean,
  `cargo clippy --all-targets --all-features -D warnings` clean),
  including 8 dual-track deletion + timbre acceptance tests.
- Frontend: **574/574 Vitest** across 56 files (550 baseline + 13 deletion
  and fiscal-settings tests + 11 starter-pack tests),
  `eslint --max-warnings 0` clean, `tsc --noEmit` clean, `vite build` clean.
- Installer: NSIS `Baraka UltraPOS_1.0.0_x64-setup.exe`; SHA-256 recorded
  in `release-v1.0.0/SHA256SUMS.txt`; delivery archive
  `Baraka-UltraPOS-v1.0.0-rc.10-Delivery.zip` checksummed alongside.

## Highlights — the 5 owner customizations

1. **Warehouse assignment on product creation** — depot `<select>` in the
   creation modal (active depots only, active depot pre-selected); initial
   stock is provisioned directly into the chosen depot.
2. **Dual-track safe deletion (ADR-092)** — zero-reference rows are
   hard-purged; historically referenced rows are soft-deactivated
   (`is_active = 0`). Guards: `DEP-01` immutable, non-zero-balance customers
   blocked, stocked depots blocked. Bilingual `SafeDeleteModal` distinguishes
   both modes across Catalog, Customers, Suppliers, and Depots.
3. **Universal instant search** — debounced instant filtering in POS,
   Catalog, Stock (`#stock-search-input`), and Suppliers (150 ms).
4. **Starter pack lifecycle (300 Algerian hardware references)** —
   one-click batch installation into the selected depot with live
   `X / 300 (Y %)` progress, dynamic install detection, and dual-track
   uninstall preserving sold items (exact hard-deleted/archived summary).
5. **Configurable fiscal stamp (Timbre)** — `fiscal.timbre_enabled` setting
   (default true) threaded through preview and checkout; Settings →
   Fiscalité toggle (ADMIN-only writes) with immediate persistence.

## Offline handbook

- `GUIDE-UTILISATION.html` carries the rc.10 badge and three new bilingual
  sections (§12 safe dual-track deletion, §13 starter pack + depots,
  §14 fiscal stamp); fully self-contained, zero network dependencies.

## Checksums

| File | Bytes | SHA-256 |
| ---- | ----- | ------- |
| `Baraka UltraPOS_1.0.0_x64-setup.exe` | 222308475 | `5512CB43106CE911A1F31659DF9000B9F41011F01B59C66005EFD04730C0DF5D` |
| `GUIDE-UTILISATION.html` | 33825 | `B2FAABF63E3A31D68A96EEB376C42E024441752E16E666DB8DE252934F3BFFAD` |
| `Baraka-UltraPOS-v1.0.0-rc.10-Delivery.zip` | 222342588 | `06A020D56FBA5814F6D4A558AF6F9D6A5C9D3369B7D20D6D396F4788DD73B4B0` |

## Open gates (owner)

- Desktop re-verification of the starter-pack install/uninstall flow and
  the fiscal toggle on real hardware.
- At-rest encryption remains an open owner decision (unchanged).
