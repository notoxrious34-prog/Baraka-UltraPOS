# Master Architecture v1.0.0 - Errata and reality check

Evidence base: AUD-003 (audit/AUDIT-REPORT-3.md) and AUD-004 (audit/AUD-004/) at HEAD 977a804. MASTER-ARCHITECTURE-v1.0.0.md is kept byte-identical to the approved copy; this file records where reality differs. Phrases such as "GO" or "ready for implementation" in the master document are historical and are not current claims.

## A. Deviations
| # | Master section | Reality | Resolution |
|---|---|---|---|
| 1 | 1.15, 26, ADR-009 (SQLCipher) | No sqlcipher in any Cargo.toml or Cargo.lock; rusqlite features: bundled, backup, modern_sqlite, hooks | OPEN OWNER DECISION (database encryption at rest) |
| 2 | 18, ADR-011 (FTS5) | No fts5 in crates, src or migrations | To be built by a BRK task (Charter search upgrade) |
| 3 | 24 (tax engine) | tax_rules never read; sale lines priced with VAT rate 0 | Superseded by ADR-068 |
| 4 | 14 (POS cockpit) | Two header rows, six fixed buttons, h-screen w-screen root, empty product panel | Superseded by ADR-066 and ADR-067 |
| 5 | 17 (locales) | English not implemented; default ar-DZ without persistence | Superseded by ADR-069 |
| 6 | 16.2 (bundled fonts) | document.fonts.size 0 and zero @font-face rules; assets/fonts holds only LICENSE-Inter.txt and LICENSE-NotoNaskhArabic.txt | Fonts to be bundled by a DSN task (Charter INIT) |
| 7 | 37 (roadmap BRK-001..013) | Real numbering reached BRK-057 | Master roadmap is historical |
| 8 | 38 (production readiness) | No release bundle in src-tauri/target/release/bundle; open gates: real printer verification, tax and legal validation, encryption decision | No readiness claim until Charter section 9.2 holds |
| 9 | tauri.conf.json | Default 1280x800, minHeight 768 exceed the 1280x689 working area | ADR-066 |
| 10 | 30 (docs/adr) | Directory did not exist | Created by TASK-REL-003 |

## B. Engineering debts carried from SESSION-BRK-011 (previous architect's package)
1. record_credit_debit in the customers repository writes the ledger entry only; current callers are disciplined, structural hardening is future work.
2. Dead column stock_levels.quantity_micro (live reads use quantity_base_micro).
3. SessionInfo does not carry the permission set; frontend gates are by role name while Rust is the authority.
4. Routing work print jobs to the capture simulator is not implemented.
5. No rotation or retention policy for the restore quarantine.
6. Calibration of PAPER_WIDTH_DOTS=576 (42 columns) for 58 mm paper (see ADR-070).
7. Environment constraint: cargo -j 2 is mandatory on the owner's machine (linker out-of-memory LNK1102).
8. VAT and timbre values are configuration seeds awaiting professional validation (see ADR-068).

## C. Not verified
Test counts in earlier packages (335 Rust, 166 Vitest) are historical. AUD-004 static counts (354 #[test] lines, 175 it/test lines) were not executed.
