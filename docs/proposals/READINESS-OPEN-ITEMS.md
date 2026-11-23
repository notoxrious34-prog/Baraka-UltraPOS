# Readiness open items — decision briefing (no code until approval)

Three roadmap items cannot be executed without owner directives. For each:
what the repo already contains (grounded), the candidate reading, and the
single question that unblocks a Bicameral TASK-* spec. **No implementation
until the owner answers.**

## 1. Idle fiscal seam (`XS`)

**In the repo:** `timbre_centimes` is a first-class money field on finalized
documents (`crates/domain/src/sales/document.rs:239`); `timbre_enabled` and
fiscal keys live behind `FiscalSettingsPort`
(`crates/application/src/ports.rs:1822`); checkout computes the stamp through
the atomic path. Nothing named "idle" or "seam" exists anywhere.
**Candidate reading:** the stamp behavior when the till sits idle across a
fiscal boundary (midnight / year rollover with an open cart or open cash
session): which `fiscal_year` and which stamp table apply — the cart's
creation moment or the finalization moment?
**Question for owner:** on New Year's night with an open cart, does the
finalized ticket take the new year's sequence + stamp (finalization
moment), or the old year's (cart creation)?

## 2. `BRK-124` counter sync

**In the repo:** gapless per-type-per-year sequences in
`document_sequences` via `allocate_in_tx`
(`sqlite_document_sequence_repository.rs`), overflow at 999_999, allocated
inside the atomic checkout transaction. Matrix/série flows create no
documents and consume no sequence numbers.
**Candidate readings:** (a) after a backup restore or external import,
`last_sequence` must resync to `MAX(existing numbers)` to avoid
collision/gap; (b) a repair/admin screen showing per-type counters with a
resync action.
**Question for owner:** is BRK-124 the post-restore resync (automatic,
inside the restore path) or an admin repair screen — or something else
entirely?

## 3. Owner reading companion (dual-mode, LAN-only)

**In the repo:** no companion app, no LAN server, no sync protocol. The
desktop is offline-first with a single SQLite file; the updater endpoint
is the only network surface.
**Candidate reading:** a read-only phone/tablet view (today's sales, cash
position, alerts) served over the shop LAN by an opt-in read-only HTTP
endpoint on the till, plus a static-export mode (PDF/WhatsApp assortment
already exists via DSN-117).
**Question for owner:** phone app on the same Wi-Fi (needs a new LAN
surface + auth story), or static PDF/WhatsApp exports only (no new
surface)? The first is a project of its own; the second is days.

## 4. Already-owner-side (no question, awaiting action)

- `.sig` key ceremony + trial install → runbook
  `docs/operations/local-signing.md` ready; key stays outside the repo.
- `PRT-06` field session → needs the physical printer on the counter.
