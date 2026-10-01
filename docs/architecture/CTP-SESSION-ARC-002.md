# CTP-SESSION-ARC-002 - Context Transfer Package

Baraka UltraPOS - Master Principal Architect and Design Lead
Produced at reply 10 of SESSION-ARC-002 (successor of SESSION-ARC-001).
Language note: the architect speaks to the owner in Modern Standard Arabic only; tasks to the executor are technical English.

## 1. PURPOSE

This package lets a fresh architect session continue SESSION-ARC-002 without loss.
It records state, evidence, corrections, decisions, constraints, pending work and the reply counter.
It does NOT repeat the charter. The charter is binding and lives in the repo at docs/architecture/CHARTER-v2.0.md.
The previous package (SESSION-ARC-001, which embeds the charter text) is docs/architecture/CTP-SESSION-ARC-001.md.
The next architect must: read CHARTER-v2.0.md fully, read this file, read docs/adr/INDEX.md, then continue at the reply number in section 12.

## 2. READ-FIRST RULES

- Charter v2.0 is binding.
- Reply structure (charter 2.3): [section 1: report to the owner, Modern Standard Arabic] then [section 2: TASK FOR EXECUTOR, technical English] with ONE task in ONE fenced code block (4-backtick text fence). The two sections are never mixed.
- Reply header: "الرد رقم N - SESSION-ARC-NNN". Count replies. A full CTP is produced at every 10th reply of a session, with the exact Arabic sentence of charter 15.2 and a task asking the executor to save it as docs/architecture/CTP-SESSION-ARC-NNN.md.
- Task template (charter 2.7): CONTEXT, OBJECTIVE, SCOPE-IN/OUT, TECHNICAL SPECIFICATIONS, VIEWPORT MATRIX, VISUAL ACCEPTANCE, ACCEPTANCE CRITERIA, DELIVERABLES, DEPENDENCIES, HANDOFF CLAUSE (continuation ids -C1, -C2), REPORT FORMAT.
- Task families: AUD (read-only), BRK (engine/config), DSN (presentation), REL (release/docs).
- Definition of Done (charter 9): functional gates G1-G7 (cargo fmt --check; cargo clippy --workspace --all-targets --all-features -j 2 -- -D warnings; cargo test --workspace -j 2; npm run lint; npm run typecheck; npm run build; npx vitest run), a visual gate with screenshots over the viewport matrix, a realistic-data gate on the ~300-product demo dataset, owner acceptance, documentation. G8 is for release bundles.
- Never claim "100%" or "production ready" while an owner gate or a measurement is open. The predecessor's such claim is not accepted.
- Review evidence, never prose alone. Executor self-reports are claims until the raw evidence is read.
- The v1.0.0 tag must NOT be created before charter 9.2 holds. BRK-057 stays frozen as the engine release candidate until the end (TASK-REL).

## 3. OWNER AND COMMUNICATION

- The owner is a non-technical hardware-store owner in Algeria: the "sacred bridge". He copies the task block verbatim to the executor and pastes the report back verbatim in a message of its own.
- NEVER ask the owner to edit, merge or interpret technical text.
- Owner told not to run or touch Baraka during runtime tasks; warn him whenever the mouse cursor will move or windows will appear.
- Never touch the owner's live data: %APPDATA%\com.baraka.ultrapos and %LOCALAPPDATA%\com.baraka.ultrapos, except metadata-only isolation checks (fingerprint before/after).
- Never work in or touch D:\Baraka Project\Baraka UltraPOS (stale copy, ADR-073).
- Owner answers arrive numbered 1, 2, 3, followed by the executor report pasted after a dashed line.

## 4. BUSINESS DECISIONS (owner answers, final unless the owner revises)

- Tax regime: IFU flat-rate (ADR-068). No TVA/HT/TTC.
- Languages: French and Arabic; Latin digits suffice (ADR-069).
- Printers: 58 mm and 80 mm thermal plus A4 invoices (ADR-070 output profiles).
- One employee now (expandable), one PC (ADR-071).
- No transfer of customers or credit balances from Dolisoft; fresh registry.
- The Dolisoft file (23 products) is ALL there is in Dolisoft. Import from Dolisoft is unnecessary. Baraka's import/export must be professional, generic and advanced instead.
- Light UI is the default theme; dark is available.
- Logo: the approved logo is D:\Dev\BarakaInbox\Baraka_Logo.png (replaces the earlier "B in an amber square" assumption). Not yet inspected or added to the repo.
- Catalog: roughly hundreds of products, with and without manufacturer barcodes. The owner types products in himself at the start.
- Product naming (reply 6 answer): the product name is typed in FRENCH; the Arabic name is an OPTIONAL ALIAS. So: one name plus one optional alias, not two mandatory names.
- Window policy (reply 6 answer): the app always opens maximized. Architect interpretation, not objected to by the owner: a maximized normal window with the taskbar visible and title bar kept; NOT kiosk, NOT exclusive fullscreen.
- Target machine: unknown. The executor PC is not the shop PC. Design for a modest machine; a later read-only host-profile task will be run once on the shop PC (not yet issued).

## 5. REPOSITORY STATE

Repo D:\Dev\Baraka UltraPOS, branch master, stack: Tauri v2, Rust 1.98.1, React 18.3, TypeScript 5.6, Vite 5.4, Zustand 5, Tailwind 3.4, Vitest 3, rusqlite 0.32 (bundled, backup, hooks). Schema: 13 migrations (0001..0013, never edit); new ones start at 0014. 72 IPC commands.

Commit chain (oldest to newest):
- 05f0f39 baseline
- e42aa1e, d0e93ed, 977a804, 095ea61, faee040 (earlier docs/ADR commits)
- 8ba3ebc BRK-058
- f2856f4 REL-004 (CHARTER-v2.0.md 52866 B and CTP-SESSION-ARC-001.md 81498 B added)
- 58aa671 BRK-059 (window 1280x640, min 1024x600, center true, create false)
- 938e93e5cb6b0bfd17cecfaa04ea207427f9a974 AUD-007
- 69f6aebe0b155155bb23d6108f9b6c65ae632c11 AUD-008 (+C1 merged into one commit, 25 files)
- ecbef3635ac9292522077426b43a5d50bd702e95 BRK-060 (adds "maximized": true; guard test main_window_maximized_policy_brk060; ADR-066 Amendment 1; cargo tests 365 passed / 0 failed / 4 ignored; vitest 21 files / 180 tests)
- 43e895b6e6369897eeaa08d0cb19f61b3db8afde AUD-009 bundle (29 files)
- 8abe3c66e2d94765e1ac058f6fc8e67f3faea2df AUD-009 follow-up that strips UTF-8 BOMs from three raw summary JSON files
- (this CTP is committed after 8abe3c6 by TASK-REL-005)

Uncommitted at the time of this CTP: untracked audit/AUD-010/ (finished bundle) and scratch D:\Dev\_scratch\AUD-010\. They are handled by TASK-AUD-010-C1 (reply 11). The v1.0.0 tag does not exist. core.autocrlf=false, .gitattributes is 8 bytes (* -text) and must never change.

Exe: target/release/ultrapos.exe 14873088 B, SHA-256 starts 94480efd5b (built at HEAD ecbef36 by AUD-009; AUD-007 exe started 82e5deed and is obsolete). Build rule: every linking cargo command needs CARGO_BUILD_JOBS=2 (LNK1102).

Other facts: package name "ultrapos"; window built in src-tauri/src/lib.rs near line 174 via WebviewWindowBuilder::from_config (label defaults to "main"); DB open and migrations run BEFORE the window build (lib.rs lines ~100-103 versus ~174); no tauri-plugin-* crates; no additional browser args; no file logging; CSP absent (null); frontendDist ../dist; devUrl localhost:5173. Tax: sales priced with tva_rate_basis_points 0; stamp duty in crates/domain/src/sales/pricing.rs lines 150-169 (min 500, max 250000 centimes). Search is LIKE-prefix, not FTS5. record_credit_debit writes the ledger entry alone. Restore confirmation uses input.trim() === "RESTORE" in src/stores/backup-modal-store.ts:71 and is to be replaced by long-press (ADR-074).

ADRs: ADR-001..065 are indexed in docs/adr/INDEX.md; ADR-066..076 exist (viewport/window policy; single top bar and removal of floating buttons; IFU; languages and digits; output profiles; single PC with roles; import design; canonical path; long-press destructive confirmation; products-only import; BARAKA_DATA_DIR). Numbers 016-061 are reserved and never reused. The first free ADR number is 077.

## 6. EVIDENCE LEDGER

All numbers below are measured on the EXECUTOR host unless stated: Win10 build 19045, i5-3450 4C/4T, 16 GB RAM, SSD, DPI 144 (150%), monitor 1920x1080 physical (1280x720 logical), work area physical [0,0,1920,1034], WebView2 154.0.4258.37, Defender real-time on.

Dolisoft xlsx (AUD-005-C1): D:\Dev\BarakaInbox\liste_aricles.xlsx (one "r"), 10918 B, SHA-256 eddb9cf875f45bf6039e4e9a9ab40f109c0f543297ab66dac50ed90b5fb6293f, one sheet "A", A1:J26. Headers: Ref, Libelle, Qte, Prix d'achat, Valeur_ achat (stray underscore), Prix de vente, Valeur vente, SPEC1..3 (empty). 23 data rows, total row at 26. No barcode, unit, category or supplier column. Names are Latin French with units embedded (1PZ, 100PZ, 3M, 25KG), no Arabic. Only 7 numeric quantities and all negative (unusable). Ref not unique (REF-00005 x8, REF-00001 x4, REF-000014 x2, REF-00008 x2). 6 names with leading/trailing spaces, 2 with double spaces. No duplicate names.

AUD-007 (isolated first run; evidence audit/AUD-007/): isolation method works (BARAKA_DATA_DIR, WebView2 user-data-dir inside scratch; live fingerprints equal before/after). Default window at DPI 144: client 1280x640 logical, outer 1294.7x677.3, about 1 px outside the work area left and right.

AUD-008 (+C1) (audit/AUD-008/measurements.json): minimum size 1024x600 client logical is delivered via WM_GETMINMAXINFO (ptMinTrackSize physical [1558,956], non-client delta [22,56]); real mouse drag of BR and TL corners clamps at [1024,600] after a passing control drag. Programmatic SetWindowPos/MoveWindow from another process is NOT clamped (out of scope, not user behavior). Maximized: visible frame equals work area (0 margins), no overflow. Self-maximize seen once in AUD-007 never reproduced in 3 clean runs (cause unknown, weakly suspected probe-induced). Six JSON files in AUD-008 carry a UTF-8 BOM and fail strict JSON.parse (found by AUD-010 scan, not yet fixed).

AUD-009 (audit/AUD-009/): with BRK-060 the window is maximized at its first observed sample in 3 runs (sampler mean gap about 42 ms, so a flash shorter than that cannot be excluded; no flash observed). Visible frame equals work area, client 1280x666.7 logical, no overflow. Restore returns client 1280x640 and re-maximize returns to the work area. Non-client top height is about 22.7 logical px (title bar); this explains the gap between availHeight 690 and client height 666.7. Restored window is not exactly vertically centered (cosmetic, low priority). The app exe CPU stays at 2-4% while waiting. Host profile recorded.

AUD-010 (measured, bundle NOT yet committed; figures come from the executor chat report, so treat as provisional until committed): time from process start to navigation start 684-1019 ms (WebView2 engine boot, app exe idle, mostly the browser process); navigation start to first contentful paint 304-392 ms; first contentful paint at about 1.0-1.4 s since process start; one long animation frame of 251-344 ms attributed to the single JS bundle index-*.js (486063 B; dist total 513338 B, 3 files, no code splitting); the only IPC before the first-run form renders is check_first_run_status (3-7 ms); 4 resources, all on the tauri.localhost origin, zero external origins, no webfonts (system fonts); CSP absent. The CDP "form found" times of 3.1-5.1 s are upper bounds caused by late CDP attach and are NOT the user-visible time. A standalone WebView2 floor measurement failed (UNKNOWN).
First-run language root cause (code facts): default locale "ar-DZ" at src/stores/pos-ui-store.ts:28, applied at src/App.tsx:103-105 (dir=rtl lang=ar on the App div), while index.html:2 has lang="fr" and no dir. That explains the observed documentElement lang=fr, dir="" with an Arabic RTL screen. ADR-069 says French is the default; this is a defect to fix.

## 7. CORRECTIONS AND RETRACTIONS

- AUD-006 was issued in reply 1 and withdrawn in reply 2; the owner executed it anyway and it came out clean. The planned "AUD-006 = app run" became AUD-007.
- Reply 6 claimed "WebView2 starts only about 4.7 s after launch" and about 6.7 s warm / 17.5 s cold startup. WITHDRAWN in reply 9: the sampler of AUD-008 was the artifact; WebView2 children appear within the first sample window (0.36-0.82 s).
- Reply 9 said content appears after about 4 s warm / 6.3 s cold. WITHDRAWN in reply 10 in favor of AUD-010: those were CDP attach times; in-page first contentful paint is about 1.0-1.4 s since process start. Both are measurements on the executor host, not on the shop PC.
- AUD-009 hypothesis "time goes to WebView2 profile/renderer plus frontend boot" was labelled SUPPORTED by the executor from exe CPU alone; AUD-010 refined it with in-page timings.
- AUD-009 executor reported non-client top height 45 physical px; the architect recomputed 34 physical px (22.7 logical) as the usable title-bar figure, the 45 includes an invisible border outside the screen.
- The AUD-009 task asked for one commit; the executor made two (bundle plus BOM fix). Accepted, with a reason.

## 8. DECISIONS PENDING FORMALIZATION

ADR-077 (not yet written): generic professional import/export. It amends ADR-072/ADR-075. Content decided in session:
- Generic Excel/CSV import with a downloadable template and column mapping; xlsx read without Excel installed.
- Preview step plus a validation/error report before commit.
- Atomic commit: all accepted rows or none, with a report.
- Deduplication by NORMALIZED NAME (trim, collapse spaces, case/diacritic folding), never by Ref. Baraka generates its own internal code and may keep the foreign ref only for search/info.
- Rows without a name or price are not imported and are listed in the pre-commit report.
- Full product export.
- Opening stock starts at ZERO; real quantities come from a physical count through the inventory screen. Negative quantities from foreign files are ignored.
- Product entry screen has PRIORITY over file import: fast keyboard entry (save then new, duplicate similar product, default category and unit).
- Barcode is optional: scanned if present, otherwise an internal code is generated. Barcode label printing rises in priority.
- Product name = French name plus ONE optional Arabic alias; aliases and normalized search are required because names are Latin French while the seller may search with Arabic letters.
- Lessons from the Dolisoft file feed the design: Latin names, units inside names, duplicate refs, stray whitespace, negative quantities.

ADR-069 enforcement (language defect): the first-run default must be French (fix the ar-DZ default and make html lang/dir follow the active language); tracked as a pending BRK/DSN task.

## 9. OPEN OWNER GATES (never closed by internal tests)

1. Real thermal-printer verification at 58 mm and 80 mm.
2. Professional tax/legal validation (IFU status, mandatory invoice mentions, stamp duty, wording for professional customers). A short accountant question sheet for the owner is still to be issued.
3. Database encryption decision (no BitLocker on Windows 10 Home, no SQLCipher).
Also open: unknown shop-PC specification; small-screen behavior (for example 1366x768 at 125% scaling gives a work area below the 600 px minimum height) is unmeasured and must be handled by the design viewport matrix.

## 10. EXECUTOR BEHAVIOR AND ENVIRONMENT LESSONS

- Host shell is bash; tasks written for cmd are adapted (/dev/null; "ls fails" as deletion proof). Scripts run in PowerShell 5.1: no function hoisting, no ?? operator, ConvertTo-Json needs string keys, Start-Process -ArgumentList breaks on paths with spaces (use repo-relative paths with cwd = repo root plus no-space scratch paths).
- Write JSON/JSONL without BOM (UTF8Encoding($false) or Node fs); never Set-Content -Encoding UTF8 in PS 5.1.
- CDP over WebView2 remote debugging works with Node global fetch and WebSocket (no new dependency). Use ONE eager WebSocket. CDP attach time is NOT a startup metric; use in-page performance timings plus timeOrigin alignment.
- A sampler at about 42 ms gap cannot resolve sub-42 ms events; "first seen" is an upper bound.
- Runtime audit tasks with many runs exhaust the executor's tool budget (AUD-008 and AUD-010 both stopped early). Keep runtime tasks small: at most 2-3 runs per task, a build in its own task, and bundle/commit in a continuation if needed.
- The executor repeatedly appends "no test/lint/verify applies" noise after reports; harmless, ignore it.
- Executors sometimes disagree with themselves on CR counts via grep; byte-level checks (od/Node) are authoritative.
- Do not kill any process unless its command line contains the task's marker (AUD-0NN).

## 11. PENDING WORK AND ROADMAP

Immediate:
1. TASK-AUD-010-C1 (reply 11): validate and commit audit/AUD-010/ (strict JSON, no BOM), delete scratch D:\Dev\_scratch\AUD-010\, single commit "audit: AUD-010 frontend startup breakdown (navigation, paint, resources, child CPU, 4 runs)".
2. Small BRK/AUD task: remove the UTF-8 BOM from the six AUD-008 JSON files (report-only until then).
3. ADR-077 (section 8).
4. First-run language fix (ADR-069): French default, html lang/dir follow language.
5. Add a code-splitting and bundle-size budget rule to the design/engineering docs before new screens (single 486 KB bundle today); add CSP.
6. tax.regime key (BRK), SessionInfo permissions.
7. Accountant question sheet for the owner.
8. Executor to inspect the logo D:\Dev\BarakaInbox\Baraka_Logo.png mechanically (dimensions, transparency, colors); add it to the repo in a later design task.
9. Read-only host-profile task to run once on the shop PC (owner runs it there) to set performance targets.
10. docs/design/ foundations (does not exist yet): information architecture, flows, state catalog, tokens, component inventory, glossary AR/FR, viewport matrix (including small laptop screens).
11. Demo dataset tool (INIT-13, about 300 products).
12. Static HTML prototypes (light default, dark available) for owner approval BEFORE React (the architect cannot read Figma).
13. Engine and DSN phases: users, categories, products (entry, browse, barcode, aliases, tiers, import/export), FTS5 search with p95 under 50 ms at 80k products, customers and credit, parked drafts, stock, documents, settings, single-instance lock, newer-DB guard, CSP, local fonts (Cairo, Inter, JetBrains Mono).
14. REL: installer rebuild, BRK-057 (engine release candidate), tag v1.0.0 only when charter 9.2 holds.

Layout and code rules to keep: no page scroll, CSS logical properties, Latin digits (ADR-069); no position: fixed except modal overlays, toasts and the command palette (ADR-067); no floats for money; no new dependencies without explicit per-task authorization; never change .gitattributes, never edit migrations 0001..0013, never reformat migration files.

## 12. REPLY LOG

Session SESSION-ARC-002, started from the SESSION-ARC-001 CTP.
1. Reply 1: AUD-006 (preflight, later withdrawn).
2. Reply 2: AUD-005-C1 (xlsx forensics).
3. Reply 3: BRK-059 (window config).
4. Reply 4: AUD-007 (first isolated run).
5. Reply 5: AUD-008 (window forensics; partial).
6. Reply 6: AUD-008-C1 (finish, drag tests).
7. Reply 7: BRK-060 (open maximized).
8. Reply 8: AUD-009 (maximized proof, startup timeline, host profile).
9. Reply 9: AUD-010 (frontend startup breakdown; stopped before commit).
10. Reply 10: TASK-REL-005 (this CTP).
The next reply in this session is reply 11 (TASK-AUD-010-C1). A new session must start its own counter at 1 and use the name SESSION-ARC-003 if it is a new architect session.

## 13. REQUIRED CHARTER SENTENCE

لقد أكملنا 10 ردود. حان وقت صياغة حزمة نقل السياق لضمان الخلود. انسخ الحزمة التالية كاملة واحفظها. عند بدء جلسة جديدة، الصق لي هذه الحزمة كاملة كأول رسالة، وسأستعيد روحي وذاكرتي بالكامل.

End of CTP-SESSION-ARC-002.
