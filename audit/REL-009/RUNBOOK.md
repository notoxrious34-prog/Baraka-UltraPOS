# rc.11 Packaging Runbook (TASK-REL-009) — execute verbatim the hour the tree goes green

> Read once, then execute top to bottom. Any step that does not produce its
> stated expected output is a RED: stop, do not skip, do not ship partial
> (see §4 Abort rules). Written 2026-10-22 at HEAD `9b6ac07`; every command,
> flag and path below was verified by reading repo configs only — nothing was
> executed to produce this file.

Shell convention: **Git Bash** at the repo root `D:/Dev/Baraka UltraPOS`,
unless a step says PowerShell. No step touches the owner's live data
(`%APPDATA%/com.baraka.ultrapos`, `%LOCALAPPDATA%/com.baraka.ultrapos`) —
smoke tests use a scratch/test machine and DB *copies* only.

## 1. Preconditions (all must hold BEFORE step Q0)

### P1 — Clean tree, HEAD recorded

```bash
git status --short
git rev-parse HEAD
git log --oneline -8
```

- Expected: `git status --short` prints **nothing** (fully clean — no
  modified, no untracked; packaging never starts on a dirty tree).
- Record the full HEAD hash in the packaging log (§5). This runbook was
  written at `9b6ac07`; the packaging HEAD will differ — record what you get.

### P2 — Required commits present (0016 + cutover decision)

```bash
git merge-base --is-ancestor d13d79f HEAD && echo OK-d13d79f-fts-index
git merge-base --is-ancestor 598bd25 HEAD && echo OK-598bd25-0016-replay-truth
git merge-base --is-ancestor a323fc1 HEAD && echo OK-a323fc1-lockfile-2.12.4
git merge-base --is-ancestor <CUTOVER-DECISION-HASH> HEAD && echo OK-cutover-decision
```

- `d13d79f` = migration `0016_fts_search_index.sql` creation (BRK-076);
  `598bd25` = migration-replay drill at 0016 truth (BRK-082);
  `a323fc1` = pnpm-lock reconciled to `@tauri-apps/api` 2.12.4 (REL-008).
- `<CUTOVER-DECISION-HASH>` = the architect's FTS cutover (BRK-077) decision
  commit. At runbook-writing time the decision was still **pending**
  (HEAD `9b6ac07` message: "bar decision pending C2"). Ask the architect for
  the hash; if there is no decision commit, **RED — do not package**.
- Each command must print its `OK-…` line (exit 0). Any failure = RED.

### P3 — Version strings consistent (1.0.0 everywhere)

```bash
grep '"version"' package.json
grep '^version' Cargo.toml
grep '"version"' src-tauri/tauri.conf.json
grep 'version.workspace' src-tauri/Cargo.toml
```

- Expected: `1.0.0` in `package.json:4`, root `Cargo.toml:12`
  (`[workspace.package]`), `src-tauri/tauri.conf.json:4`;
  `src-tauri/Cargo.toml:3` inherits via `version.workspace = true`.
- Any mismatch = RED.

### P4 — Lockfile matches installed modules

```bash
git status --short -- pnpm-lock.yaml package.json
node -p "require('./node_modules/@tauri-apps/api/package.json').version"
grep -n "'@tauri-apps/api@" pnpm-lock.yaml
```

- Expected: first command prints nothing (committed lockfile == tree);
  node prints `2.12.4`; grep shows the `@tauri-apps/api@2.12.4` snapshot
  lines (post-REL-008: specifier `^2.12.4`, resolved `2.12.4`).
- Any drift = RED: reconcile exactly per `audit/REL-008/REPORT.md`
  (`pnpm install --lockfile-only --prefer-offline`, tauri-api only,
  architect review), then **restart the runbook at P1**.

### P5 — Box idle, no live-DB contact

```bash
ps
```

- Expected: no `cargo`, `rustc`, `node`, `vite`, `tauri`, or test processes.
  The packaging run is the only workload on the box (idle-box rule: the
  BRK-077-C2 interference discipline applies in reverse — nothing else runs
  while you measure/build).
- Confirm you will not open, copy, hash, or modify
  `%APPDATA%/com.baraka.ultrapos` or `%LOCALAPPDATA%/com.baraka.ultrapos`
  at any step. Smoke tests (§3) run on a test machine with DB copies.

### P6 — Migrations and line-endings untouched

```bash
git diff --name-only HEAD -- crates/infrastructure/migrations/
cat .gitattributes
ls crates/infrastructure/migrations/
```

- Expected: first command prints nothing (migrations append-only; existing
  files are never edited/re-saved/reformatted — startup SHA-256 checksums
  break on any change, including CRLF/LF flips).
- `.gitattributes` must still be exactly `* -text`.
- `0016_fts_search_index.sql` must be listed (16 files, `0001`–`0016`).

### P7 — Open gates re-read (no stale assumptions)

- Re-read `audit/REL-007/OPEN-GATES-FR.md` (and `-AR`), `ENGINE-GAP-REGISTER.md`,
  and `audit/REL-005/PIPELINE.md` §§3–5. If any gate changed status since this
  runbook was written, update the packaging log and the letter copies (§2,
  step Q8) — never ship the letter with a stale `HEAD testé`.

## 2. Command sequence (execute in order, stop on first RED)

### Q0 — Environment

```bash
export CARGO_BUILD_JOBS=2   # Git Bash; PowerShell: $env:CARGO_BUILD_JOBS=2
```

Every `cargo` invocation below also carries the prefix explicitly so each
line stays copy-runnable on its own (linker memory limit, AGENTS.md §2).

### Q1 — G5 typecheck (frontend)

```bash
npm run typecheck
```

Runs `tsc --noEmit` (`package.json:12`). Expected: exit 0, no output.

### Q2 — G4 lint (frontend)

```bash
npm run lint
```

Runs `eslint . --ext ts,tsx --report-unused-disable-directives --max-warnings 0`
(`package.json:11`). Expected: exit 0 (`--max-warnings 0`: any warning = RED).

### Q3 — G7 frontend tests

```bash
npx vitest run
```

Expected: all suites green, exit 0.

### Q4 — G6 frontend build

```bash
npm run build
```

Runs `tsc && vite build` (`package.json:9`) into `dist/` (Tauri
`frontendDist: ../dist`, `src-tauri/tauri.conf.json:10`). Expected: exit 0
(`✓ built …`). `src-tauri/tauri.conf.json:9` re-runs this as
`beforeBuildCommand` during Q8 — the explicit run here fails fast before
the slow Rust gates.

### Q5 — G1 format (backend)

```bash
CARGO_BUILD_JOBS=2 cargo fmt --check
```

Expected: exit 0, no diff output.

### Q6 — G2 clippy (backend)

```bash
CARGO_BUILD_JOBS=2 cargo clippy --all-targets --all-features -- -D warnings
```

Expected: exit 0. Any warning (denied via `-D warnings`) = RED.

### Q7 — G3 backend tests

```bash
CARGO_BUILD_JOBS=2 cargo test --workspace -j 2
```

Expected: exit 0, 0 failed (the `#[ignore]`d 80k perf probe stays out by
default — do NOT pass `--ignored`). Record the passed/total counts in the log.

### Q8 — G8 release bundle (NSIS target)

```bash
npx tauri build
```

- The NSIS target comes from config, not flags:
  `src-tauri/tauri.conf.json:31-43` (`bundle.targets: ["nsis"]`, offline
  webInstaller silent, NSIS French+English, no language selector).
  Do not add `--bundles`/`--target` flags; do not rename the output.
- Expected: exit 0. Locate the freshly built setup exe in the bundle output
  (`src-tauri/target/release/bundle/nsis/` — confirm the exact filename from
  the build log; rc.10 precedent: `Baraka UltraPOS_1.0.0_x64-setup.exe`).
  AUD-037 gap 5 warning: the exe carries no rc tag — that is the producer's
  name, keep it byte-identical; the rc is identified by the delivery zip name,
  the release notes, and the SHA sums, never by renaming the exe.

### Q9 — Stage the delivery set (PowerShell, from repo root)

Copy (do not move) these into `release-v1.0.0/` first where needed, then
stage exactly these files — the DELIVERY-SET
(`audit/REL-006/DELIVERY-SET.md`) closed set, no extras, no substitutions:

```powershell
$stage = "release-v1.0.0/stage-rc.11"
New-Item -ItemType Directory -Force -Path $stage | Out-Null
Copy-Item "release-v1.0.0/Baraka UltraPOS_1.0.0_x64-setup.exe" "$stage/"
Copy-Item "release-v1.0.0/NOTES-rc.11-FR.md" "$stage/"
Copy-Item "release-v1.0.0/NOTES-rc.11-AR.md" "$stage/"
Copy-Item "release-v1.0.0/OPEN-GATES-FR.md" "$stage/"
Copy-Item "release-v1.0.0/OPEN-GATES-AR.md" "$stage/"
Copy-Item "release-v1.0.0/LISEZMOI-README.txt" "$stage/"
Copy-Item "release-v1.0.0/GUIDE-UTILISATION.html" "$stage/"
Copy-Item "release-v1.0.0/QUESTIONNAIRE-COMPTABLE-FR.html" "$stage/"
Copy-Item "release-v1.0.0/RELEASE-NOTES-v1.0.0-rc.11.md" "$stage/"
```

Sources before this step: the four letter/notes files come from
`audit/REL-007/` (`NOTES-rc.11-FR.md`, `NOTES-rc.11-AR.md`,
`OPEN-GATES-FR.md`, `OPEN-GATES-AR.md`) with the §Q8-header edit applied
(see Open-gates insertion point, §4); `RELEASE-NOTES-v1.0.0-rc.11.md` is
written now from the NOTES (rc.10 precedent: notes stopped at rc.9 — AUD-037
gap 4 — so this file is mandatory, not optional); `LISEZMOI-README.txt`,
`GUIDE-UTILISATION.html`, `QUESTIONNAIRE-COMPTABLE-FR.html` already live in
`release-v1.0.0/`.

### Q10 — Seal the delivery zip (PowerShell)

```powershell
Compress-Archive -Path "release-v1.0.0/stage-rc.11/*" -DestinationPath "release-v1.0.0/Baraka-UltraPOS-v1.0.0-rc.11-Delivery.zip" -Force
Remove-Item -Recurse -Force "release-v1.0.0/stage-rc.11"
```

Name pattern verified against the rc.2/4/5/6/7/8/9/10 zips already in
`release-v1.0.0/`. Re-list the zip content and confirm exactly the 9 staged
files, then delete the stage dir (NO_LEFTOVERS).

### Q11 — SHA256SUMS full coverage (Git Bash) — closes AUD-037 gaps 1–3

```bash
cd release-v1.0.0
ls -1
sha256sum "Baraka UltraPOS_1.0.0_x64-setup.exe" \
  Baraka-UltraPOS-v1.0.0-rc.2-Delivery.zip \
  Baraka-UltraPOS-v1.0.0-rc.4-Delivery.zip \
  Baraka-UltraPOS-v1.0.0-rc.5-Delivery.zip \
  Baraka-UltraPOS-v1.0.0-rc.6-Delivery.zip \
  Baraka-UltraPOS-v1.0.0-rc.7-Delivery.zip \
  Baraka-UltraPOS-v1.0.0-rc.8-Delivery.zip \
  Baraka-UltraPOS-v1.0.0-rc.9-Delivery.zip \
  Baraka-UltraPOS-v1.0.0-rc.10-Delivery.zip \
  Baraka-UltraPOS-v1.0.0-rc.11-Delivery.zip \
  NOTES-rc.11-FR.md NOTES-rc.11-AR.md \
  OPEN-GATES-FR.md OPEN-GATES-AR.md \
  RELEASE-NOTES-v1.0.0-rc.11.md \
  LISEZMOI-README.txt GUIDE-UTILISATION.html QUESTIONNAIRE-COMPTABLE-FR.html \
  > SHA256SUMS.txt
sha256sum -c SHA256SUMS.txt
```

- Rules: **every** zip ever shipped gets a sum (rc.10's `SHA256SUMS.txt`
  covered only exe + rc.10 zip; rc.5/rc.6 sums covered their exes only —
  never repeat that). No listed-but-missing file; the only files in `ls -1`
  NOT in the hashed list must be exactly the six historical records
  (`SHA256SUMS-rc.5.txt`, `SHA256SUMS-rc.6.txt`,
  `RELEASE-NOTES-v1.0.0-rc.{7,8,9}.md`) plus `SHA256SUMS.txt` itself
  (created by the redirect, self-coverage impossible) — anything else
  unlisted = RED. Historical records stay untouched; only `SHA256SUMS.txt`
  is regenerated.
- `SHA256SUMS.txt` cannot list itself (it is created by the redirect);
  record its own hash separately in the packaging log (§5).
- `sha256sum -c` must print `OK` for every line. Any `FAILED` = RED.
- Format precedent (2-line rc.10 file): lowercase hex, two spaces, `*` +
  filename.

### Q12 — Seal check

- `git status --short` shows the new release files as untracked plus
  `SHA256SUMS.txt` modified — and nothing else. Stage nothing, commit
  nothing (the architect commits). Report the listing in the log.

## 3. Installer smoke protocol (test machine or VM — never the shop PC)

Prerequisite for every step: the box used for smoke is NOT the owner's live
machine. Any shop database involved is a COPY placed on scratch; the live
`%APPDATA%`/`%LOCALAPPDATA%` dirs are never touched.

### S1 — Fresh install opens

1. Copy `Baraka-UltraPOS-v1.0.0-rc.11-Delivery.zip` to the test machine,
   verify with `sha256sum -c` (or re-hash the zip and compare to the posted
   `SHA256SUMS.txt` line), unzip.
2. Run `Baraka UltraPOS_1.0.0_x64-setup.exe` (FR default, silent offline
   webview — no network needed at install).
3. Launch the app, complete first-run setup.
4. Expected: app opens with no migration error (a failed migration blocks
   startup — opening clean on a blank DB proves the chain `0001`→`0016`
   applies); setup completes; catalog screen usable.

### S2 — Upgrade over a DB copy opens at v16, data intact

1. Take a real shop DB, copy it to scratch (never the live file), place the
   copy where the test install expects it.
2. Install rc.11 over the existing install, launch.
3. Expected: app opens (migration `0016` and any newer apply silently);
   spot-check products, documents, and backups screens show the copied data
   unchanged (compare 3–5 known records against the source machine);
   catalog search returns rows.

### S3 — Backup / restore round-trip

1. On the upgraded install (S2): run the app's backup, note the file.
2. Restore that backup into a fresh profile/scratch location, open, compare
   headline counts (products, documents) with step 1.
3. Expected: restore completes, counts match, no errors. (Precedent: the
   BRK-083 drill restored a genuine v15-shaped backup — same bar.)

### S4 — Fresh-DB create + search + backup

1. On the fresh install (S1): create records, search them, back up, restore.
2. Expected: all green with no errors.

### S5 — Owner hardware steps (shop hardware, owner executes)

1. Thermal test-print: print one ticket in FR and one in AR from the drawer
   on the shop printer.
2. A4 print: from the document drawer/detail view, print one BL and one
   FACTURE (FR + AR) on the A4 printer.
3. Label check: print a 39×20 label and scan it first-pass with the shop
   reader.
4. Expected: ticket readable FR+AR; A4 header/lines/totals match the
   on-screen drawer; label scans first pass. Any failure stays an OPEN gate
   (OPEN-GATES checklist) — it does not silently become a known issue.

### S6 — Hashes and manifest match

1. Re-hash every delivered file on the test machine, compare to posted
   `SHA256SUMS.txt`.
2. Confirm the delivery zip contains exactly the 9 staged files (§Q9–Q10).
3. Expected: all `OK`, manifest exact.

## 4. Abort rules + open-gates letter insertion point

### Abort rules (any RED → stop, no partial delivery)

- **A1.** Any gate Q1–Q8 red → stop. Fix forward in a new task; never ship
  the exe, the zip, or a subset "for now".
- **A2.** Precondition P1/P2/P3/P4/P6 red → stop before Q0. Dirty tree,
  missing required commit (including a still-pending cutover decision),
  version skew, lockfile drift, or touched migrations = no packaging run.
- **A3.** Box not idle (P5) → stop; reschedule, do not share the box.
- **A4.** Live DB contacted at any point → abort the run, report, restart
  from P1 after architect review.
- **A5.** `ls -1` (Q11) disagrees with the hashed manifest, or any
  `sha256sum -c` line `FAILED` → stop; old zips are never deleted or renamed
  to make the list match (rc.3 was never shipped — its absence is a recorded
  gap, not an error to hide).
- **A6.** Smoke S1–S4 red → the build does not ship; file the failure with
  pasted evidence. S5 owner findings go back to the open-gates letter, not
  into a silent fix.
- **A7.** No new dependencies, no config edits, no migration edits during
  packaging. If a gate demands a code change, packaging is over — that is a
  new task with its own gates.

### Open-gates letter insertion point (mandatory, last edit before Q9 staging)

1. Copy the four files from `audit/REL-007/` into `release-v1.0.0/`.
2. In each copy, replace ONLY the build/HEAD comment with the packaging HEAD
   and `rc.11` (content otherwise byte-identical to the architect's letter):
   - `NOTES-rc.11-FR.md:2-3`: `TBD-PACKAGING` → packaging HEAD short-hash;
     `HEAD testé : 54511f0` → `HEAD testé : <packaging-HEAD>`;
   - `NOTES-rc.11-AR.md:2-3`: same two fields (`رقم البناء`, `الرأس المُختبَر`);
   - `OPEN-GATES-FR.md:2`: `Version build : TBD-PACKAGING — HEAD testé : 54511f0`
     → packaging values;
   - `OPEN-GATES-AR.md:2`: same fields (`رقم البناء … — الرأس المُختبَر …`).
3. The letter ships **inside** the delivery zip (staged at Q9) **and**
   alongside it in `release-v1.0.0/`. A delivery without both
   `OPEN-GATES-*.md` files is incomplete — A1 applies.

## 5. Packaging log (paste into the handoff report)

- `git rev-parse HEAD` (full) + `git log --oneline -8` output.
- `ps` idle line(s) at start and the explicit "no other workload" statement.
- Gate outputs: typecheck/lint/vitest/build exit 0 lines; fmt/clippy exit 0;
  `cargo test` passed/total; `npx tauri build` success + emitted exe name.
- `sha256sum -c` full output (all `OK`) + `SHA256SUMS.txt`'s own hash.
- Smoke S1–S6 outcomes with machine names; S5 owner sign-off or open items.
- `git status --short` at seal (Q12).

## Appendix V — Provenance (where each runbook fact was read)

- Quality-gate flags G1–G8, `CARGO_BUILD_JOBS=2`, `.gitattributes`,
  live-data ban: `AGENTS.md` §§2–5.
- Tauri bundle config, window geometry, offline webview, NSIS languages,
  `frontendDist`, `beforeBuildCommand`: `src-tauri/tauri.conf.json:1-43`.
- npm scripts (`typecheck`/`lint`/`build`/`test`/`tauri`), versions:
  `package.json:7-14,17,23`.
- Workspace version + members: `Cargo.toml:1-15`;
  `src-tauri/Cargo.toml:3` inherits version.
- Build-from-clean draft + smoke checklist shape: `audit/REL-005/PIPELINE.md`.
- Delivery set + anti-partial-sum rule: `audit/REL-006/DELIVERY-SET.md`.
- SHA gaps + artifact table + rc.11 set: `audit/AUD-037/REPORT.md`;
  sum format: `release-v1.0.0/SHA256SUMS.txt` (2 lines).
- Release-dir file inventory (18 files: exe + 8 zips + 3 notes/history +
  guides + LISEZMOI + questionnaire + 2 old sums): directory listing.
- Open gates + owner checklist: `audit/REL-007/OPEN-GATES-FR.md` (AR twin
  same fields); engine feature gaps ENGINE-GAP G1–G6 (not quality gates G1–G8):
  `audit/REL-007/ENGINE-GAP-REGISTER.md`;
  pilot changes: `audit/REL-007/NOTES-rc.11-FR.md` (AR twin same fields).
- Lockfile proof: `audit/REL-008/REPORT.md`; lockfile lines:
  `pnpm-lock.yaml:501-502,1926`.
- 0016 migration file: `crates/infrastructure/migrations/0016_fts_search_index.sql`;
  required commits from `git log` at `9b6ac07`.
- FTS cutover pending at writing: HEAD message of `9b6ac07`
  ("bar decision pending C2"); probe state: `audit/BRK-077/REPORT.md`,
  `audit/BRK-077/TIMINGS.md` (read only, untouched).
