# Release Notes — Baraka UltraPOS v1.0.0-rc.11

- Release Candidate: **v1.0.0-rc.11**
- Date: 2026-10-24
- Git tag: `v1.0.0-rc.11` (intended; NOT yet created — the architect tags.
  `v1.0.0` remains prohibited by Charter §3.1)
- Baseline: commit `ea9d7e7` (ADR-102 search performance tiers;
  code bytes identical to `991251b`, FTS5 search cutover with
  rank-then-hydrate)

## What changed since rc.10

- Role-based access hardened: every screen (Customers, Stock, Catalogs,
  Documents, Settings, Transfers) shows only what the user's role allows;
  forbidden actions render greyed with an explanation instead of failing
  on click. *(Sources: audit/DSN-077 through DSN-081 REPORTs.)*
- Safe deletions: every deletion now requires a reason (3+ characters);
  irreversible deletions require retyping the item code; the main
  warehouse is protected. *(Source: audit/REL-007/NOTES-rc.11-FR.md.)*
- FTS5 catalog search ACTIVE: the FTS index + rank-then-hydrate cutover
  shipped in this baseline (`991251b`; migrations `0001`→`0016` apply
  cleanly on fresh and upgraded databases). Search performance tiers
  restated in `docs/adr/ADR-102` (Tier S/M gated and met, Tier-L monitored).
- Known-stale pilot-letter line: `OPEN-GATES` item 5 still says the FTS
  index is "NON activé" — superseded by the cutover above. The letter ships
  headers-corrected only (runbook rule); the architect owns the item-5
  correction before owner trial.

## Open gates (pilot — read before installing)

See `OPEN-GATES-FR.md` / `OPEN-GATES-AR.md` in this delivery set:
real-ticket printing, 39×20 label scan, tax validation, no-encryption
decision — all owner-carry, none validated.
