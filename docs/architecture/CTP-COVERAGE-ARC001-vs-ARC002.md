# CTP-COVERAGE-ARC001-vs-ARC002 — read-only mechanical coverage table

Method: a throwaway Node script (outside the repo, not committed) compared byte-identical
texts: `docs/architecture/CTP-SESSION-ARC-001.md` (555 lines, 81498 B),
`docs/architecture/CTP-SESSION-ARC-002.md`, the repo-docs corpus
(`docs/adr/*.md` + `docs/architecture/CHARTER-v2.0.md` + other `docs/architecture/*.md`
except the two CTPs), and working-tree existence for paths.
Detection rule for sections: markdown `#` headings of any level.
Charter region (verified byte-substring, see 2.1) is ARC-001 lines 15-331 and is
excluded from the inventory; section `0)` therefore spans lines 8-14 only.
A mechanical check proves TOKEN presence, not that meaning was preserved.
Line numbers below are ARC-001 1-based lines unless stated as ARC-002.

## Summary

- Embedded charter: whole CHARTER-v2.0.md (317 lines) is a byte-substring of ARC-001
  at ARC-001 lines 15-331, delimited by `<!-- CHARTER-V2.0-BEGIN -->` + ````text fence
  (lines 13-14) and ```` + `<!-- CHARTER-V2.0-END -->` (lines 332-333).
- Non-charter sections in ARC-001: 12. Classifications: COVERED-ARC002 7,
  SUPERSEDED 4, COVERED-REPO 0, NOT-COVERED 0, UNKNOWN 1.
- Identifiers extracted from non-charter text: sha256 1, paths 34, ADR ids 14,
  task ids 12, commit hashes 8, INIT ids 1, TASK-* words 2. Total distinct: 72.
- Candidates NOT FOUND ANYWHERE (ARC-002 count 0, repo-docs count 0,
  paths also missing on disk): 9 (all listed in 2.3, none truncated).

## 2.1 Embedded-charter test

Normalization CRLF->LF applied in memory (all three files already contain 0 CR bytes).
(a) Whole charter text is a substring of ARC-001: YES (byte offset 402).
(b) Not applicable (fraction = 1.0, no missing lines).
Region: ARC-001 lines 15 (`# APPOINTMENT CHARTER ...`) to 331 (`— End of Charter v2.0 —`).
Delimiters: HTML comments + 4-backtick `text` fence as above.
Headings around it: `## 0) ...` (line 8, charter pointer section) before;
charter's own `## 0...17` headings (lines 23-326) inside; `## 1) ...` (line 345) after.

## 2.2 Inventory of ARC-001 outside the charter (line: title — section lines)

- L1: `# ... SESSION-ARC-001` — 1 line (title only)
- L2: `## Baraka UltraPOS ...` — 6 lines (2-7, stamp + next-session pointer)
- L8: `## 0) ...` — 7 lines (8-14, details/summary + charter delimiters)
- L345: `## 1) ...` — 9 lines (345-353, session identity)
- L354: `## 2) ...` — 34 lines (354-387, executive summary)
- L388: `## 3) ...` — 24 lines (388-411, ADR log)
- L412: `## 4) ...` — 44 lines (412-455, roadmap and tasks)
- L456: `## 5) ...` — 24 lines (456-479, file map)
- L480: `## 6) ...` — 17 lines (480-496, technical state)
- L497: `## 7) ...` — 27 lines (497-523, errors and lessons)
- L524: `## 8) ...` — 20 lines (524-543, human context, Arabic prose, no identifiers)
- L544: `## 9) ...` — 12 lines (544-555, boot instructions)
(Titles abbreviated; full Arabic titles are in ARC-001 at the cited lines.)

## 2.3 Identifier cross-check

Counts per category (distinct identifiers in non-charter text): sha256 1, paths 34,
ADR 14, tasks 12, commits 8, INIT 1, TASK-words 2.
NOT FOUND ANYWHERE (candidates), with ARC-001 line numbers (9 total, all shown):

- L427 [sha256] `e65c3427e8ecc76d0d6e93e06be926bd723947532410e0b2330ead04b0c95962` (001x1)
- L419 [task] `REL-001` (001x2)
- L420 [task] `REL-002` (001x1)
- L423 [commit] `8ba3ebc321521b4672faa1d817e88bda5a39dd39` (001x1; short form `8ba3ebc` exists in ARC-002/repo)
- L435 [path] `D:\Dev\BarakaInbox\liste_articles.xlsx` (001x1, on-disk: no; actual inbox file is `liste_aricles.xlsx`)
- L464 [path] `persistence/mod.rs` (001x1, on-disk: no)
- L465 [path] `crates/infrastructure/migrations/0001..0013` (001x1, on-disk: no as a literal path)
- L551 [taskword] `TASK-AUD-005-C1` (001x1)
- L552 [taskword] `TASK-BRK-059` (001x1)

## 2.4 Section classification (one row per section; at most 3 evidence tokens)

- L1 | title | 1 | COVERED-ARC002 | ARC-002 §12 reply log; token `SESSION-ARC-001` (001-L1, 002x5)
- L2 | stamp+pointer | 6 | COVERED-ARC002 | ARC-002 §12; token `SESSION-ARC-002` (001-L4, 002x5)
- L8 | charter pointer | 7 | SUPERSEDED | ARC-002 §1 replaces embedded delivery with repo pointer; phrase `CHARTER-V2.0` (001-L13 marker, 002-§1)
- L345 | session identity | 9 | COVERED-ARC002 | ARC-002 §12; token `SESSION-ARC-001`
- L354 | executive summary | 34 | COVERED-ARC002 | ARC-002 §5 repo state; token `BRK-058` (001, 002-L58)
- L388 | ADR log | 24 | COVERED-ARC002 | ARC-002 §6 evidence; token `ADR-069` (001, 002-L89)
- L412 | roadmap/tasks | 44 | SUPERSEDED | ARC-002 §11 is the new roadmap; token `INIT-13` (001-L449, 002-L148)
- L456 | file map | 24 | SUPERSEDED | ARC-002 §5 new repo state/commit chain; token `05f0f39` (001, 002-L56)
- L480 | technical state | 17 | COVERED-ARC002 | ARC-002 §5 other facts; token `pricing.rs` (001, 002-L72)
- L497 | errors/lessons | 27 | COVERED-ARC002 | ARC-002 §5; token `ADR-074` (001, 002-L72). Note: topical home would be §7/§10, which share no verified token.
- L524 | human context | 20 | UNKNOWN | Arabic prose, zero identifiers; topical overlap with ARC-002 §3 is unprovable mechanically.
- L544 | boot instructions | 12 | SUPERSEDED | ARC-002 §12 new boot (reply 11, new-session rule); token `SESSION-ARC-002` (001, 002x5)

## Candidates for a future CTP revision

NOT-COVERED sections: (none).
UNKNOWN sections: L524 `## 8) ...` (20 lines, 524-543).
Identifier candidates: the 9 items listed in 2.3 (L419, L420, L423, L427, L435, L464, L465, L551, L552).
