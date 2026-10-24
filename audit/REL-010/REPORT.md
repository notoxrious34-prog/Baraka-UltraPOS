# TASK-REL-010 REPORT — rc.11 packaging (BLOCKED at Q8: WebView2 bootstrapper download hang)

Charter v2.0 binding. Runbook: audit/REL-009/RUNBOOK.md followed verbatim.
No commit, no tag. No production code changed. Tree left clean.

## (1) HEAD + clean proof

- `git rev-parse HEAD` = `ea9d7e7f998a5067337213fb91f5957bf782af04`
- `git log --oneline -3`: `ea9d7e7 docs(arch): ADR-102 search tiers with 0017 revert record` ·
  `991251b feat(engine): FTS5 search cutover with rank-then-hydrate` ·
  `e5609f7 docs(release): TASK-REL-009 rc.11 packaging runbook`
- `git status --short` at Step 0: **empty** (clean). Re-verified empty after
  the aborted build. Build artifacts (`target/`, `dist/`, `node_modules/`)
  are gitignored — no tracked change at any step.
- Idle at start: no cargo/rustc/node/tauri processes (only the pre-existing
  unrelated `openclaw` gateway node.exe — not mine, left untouched).

## (2) Preconditions P1-P7

- P1 clean + HEAD recorded: PASS (above).
- P2 required commits: `d13d79f` OK, `598bd25` OK, `a323fc1` OK
  (all `merge-base --is-ancestor` exit 0). Cutover decision: satisfied by
  `991251b` (FTS cutover commit, ancestor of HEAD).
- P3 version 1.0.0 in package.json / Cargo.toml / tauri.conf.json +
  `version.workspace = true`: PASS.
- P4 lockfile: `git status` clean for pnpm-lock/package.json; installed
  `@tauri-apps/api` = 2.12.4; lock snapshots `@tauri-apps/api@2.12.4`: PASS.
- P5 box idle, no live-DB contact: PASS (idle; never touched %APPDATA%).
- P6 migrations untouched (git diff empty), `.gitattributes` = `* -text`,
  16 migration files `0001..0016` (0017 correctly reverted per ADR-102):
  PASS.
- P7 open gates re-read (OPEN-GATES FR/AR, ENGINE-GAP-REGISTER, PIPELINE
  §§3-5): no gate status change since runbook; the OPEN-GATES letter item 5
  ("recherche rapide … NON activé") is now STALE — FTS is activated at HEAD
  (cutover in 991251b). **Deviation (quoted, per runbook §P7):** the letter
  item 5 must be corrected to reflect the activated index before the letter
  ships; flagged as an owner-carry edit, not applied here (runbook's letter
  edit covers only the build/HEAD comment lines, §4). No RED — proceed.

## (3) Gate table G1-G7 (all green) — Q8 RED

| gate | command | result |
|---|---|---|
| G5 typecheck | `npm run typecheck` | exit 0 |
| G4 lint | `npm run lint` | exit 0 (`--max-warnings 0`) |
| G7 vitest | `npx vitest run` | 75 files / 752 tests passed, exit 0 |
| G6 build | `npm run build` | exit 0 (`✓ built in 13.62s`) |
| G1 fmt | `CARGO_BUILD_JOBS=2 cargo fmt --check` | exit 0 |
| G2 clippy | `cargo clippy --all-targets --all-features -- -D warnings` | exit 0 |
| G3 test | `cargo test --workspace -j 2` | 64 suites ok, 491 passed / 0 failed, exit 0 |
| G8 bundle | `npx tauri build` | **RED — WebView2 bootstrapper download hung (aborted)** |

## (4) Q8 blocker — evidence (abort, no fix-forward)

- `npx tauri build` compiled clean (`Finished release in 10m 28s`,
  `Built application at target/release/ultrapos.exe`), then printed
  `Info Patching … nsis` → `Downloading https://go.microsoft.com/fwlink/?linkid=2124701`
  (the WebView2 runtime bootstrapper, required by the `offlineInstaller`
  NSIS bundle) and **stalled ~29 minutes** with zero bytes written:
  the cache dir `%LOCALAPPDATA%\tauri\x64\d6c50665-…\` stayed empty (checked
  repeatedly), no new file under `%LOCALAPPDATA%\tauri\` or temp dirs.
- Node process held two ESTABLISHED 443 sockets but no file appeared —
  a hung download, not slow progress. Independent probe:
  `curl -L --max-time 20` of the same fwlink returned `HTTP=200` but only
  **31 MB in 20 s** before my own timeout — the box's path to
  mp.microsoft.com / delivery.mp.microsoft.com is stalling mid-transfer.
- I killed ONLY my build processes (`taskkill /PID 1840 /T /F` — the tauri
  CLI; the npx wrapper had already exited). The unrelated `openclaw` node
  gateway was left running.
- No NSIS installer was produced (`target/release/bundle/nsis/` contains
  only the stale rc.10-era exe from Oct 17 — NOT a fresh rc.11 artifact).
- **No workaround applied** (per runbook A7 + handoff "no fix-forward"): the
  bootstrapper is a hard requirement of the offlineInstaller NSIS target,
  and skipping/changing the bundle config is a config edit (forbidden).
- Post-abort: tree clean, no strays, box idle.

## (5) Smoke S1-S6

NOT REACHED (Q8 aborted the run before staging). S1-S5 = PENDING; S5 owner
steps (thermal test-print, 39×20 scan, tax validation, no-encryption
decision) remain owner-carry, never claimed.

## (6) STATE / DONE / REMAINING

STATE blocked (Q8 red — WebView2 bootstrapper download hung; no partial
delivery per A1). DONE: P1-P7 verified, G1-G7 all green (full counts above),
abort executed cleanly with evidence, tree left clean, logs kept under
audit/REL-010/. REMAINING: re-run `npx tauri build` once the box's path to
the Microsoft CDN is healthy (retry when the network recovers — the compile
cache is warm, so it resumes at the NSIS step), then Q9-Q12 + S1-S6.
Recommendation to Architect: the offline-installer NSIS build is
network-dependent; on a firewalled build box this will recur — consider a
one-time manual WebView2 bootstrapper seed or a whitelisted mirror.
NEXT READY: NO (blocked on network).

## (7) C1 retry — Q8 red again (bundler's own timeout; no second retry)

- Step 0 re-verified: HEAD `ea9d7e7…`, status shows only `?? audit/REL-010/`,
  idle (no cargo/rustc/makensis/tauri/npx procs). G1-G7 NOT re-run per
  task (all green by fresh record, no source touched).
- Q8 retry (`npx tauri build`, warm cache, log `tauri-build2.log`): frontend
  rebuilt in 8.51s, release compile clean, NSIS step printed the same
  `Downloading …/fwlink/?linkid=2124701` line, then the bundler itself
  failed: `failed to bundle project: timeout: global`, **Q8-EXIT=1**.
- Byte log (WebView2 cache dir
  `%LOCALAPPDATA%\tauri\x64\d6c50665-…\`, timestamps + bytes — zero growth):
  T+2min 0 B · T+4min 0 B · T+6min 0 B · T+8min 0 B (dir created in the
  prior session, still empty). No kill was needed this time — the process
  exited on its own; verified NO-BUILD-STRAYS after.
- Per the C1 stall rule: STOP, no second retry this session. Tree clean
  (only `audit/REL-010/` untracked: g3.log, tauri-build.log,
  tauri-build2.log, REPORT.md).
- STATE blocked/network with byte log. DONE: Q8 retried on warm cache with
  timestamped evidence; the Microsoft CDN path stalls before a single byte
  lands. REMAINING: seed the ~150–214 MB
  `MicrosoftEdgeWebView2RuntimeInstallerX64.exe` into the cache dir
  out-of-band (or build when the CDN path recovers), then re-run Q8 → Q9-Q12
  + S1-S6. Recommendation from REL-010 stands and is now confirmed twice.
  No commit, no tag, no production change. NEXT READY: NO (blocked on network).

## (8) C2 — SEED + Q8 GREEN, Q9-Q12 sealed, delivery complete

- Step 0 re-verified: HEAD `ea9d7e7…`, status only `?? audit/REL-010/`,
  idle. G1-G7 NOT re-run (green by record, no source touched).
- Inbox file: `MicrosoftEdgeWebView2RuntimeInstallerX64.exe`,
  **214,823,632 bytes**,
  SHA-256 `9b0142e22e8daa92855d746ae4d1e9eeef561ccaf67d1dd452165c4af3df3191`
  (record only — no external comparison available).
- Name verification: expected `MicrosoftEdgeWebView2RuntimeInstallerX64.exe`
  — proven by the three sibling cache dirs (each holding that exact name)
  and the redirect target seen in the failed run. No guessing, no rename.
- SEED: COPY (inbox original intact, same size + timestamp) into
  `%LOCALAPPDATA%\tauri\x64\d6c50665-06a3-4ebe-8bde-7e408a78f093\`;
  post-copy size **214,823,632 = 214,823,632 bytes** ✓.
- Q8 (`npx tauri build`, log `tauri-build3.log`): NO `Downloading` line —
  the bundler found the seeded bootstrapper and skipped the fetch.
  `Finished 1 bundle`, **Q8-EXIT=0**. Fresh exe
  `target/release/bundle/nsis/Baraka UltraPOS_1.0.0_x64-setup.exe`,
  **223,014,545 bytes** (2026-10-24 11:44).
- Letter insertion point: 4 files copied from `audit/REL-007/`
  (byte-identical verified by `cmp`), headers-only edit
  (`TBD-PACKAGING`→`v1.0.0-rc.11`, `54511f0`→`ea9d7e7` FR+AR); bodies
  proven identical past the header lines. `RELEASE-NOTES-v1.0.0-rc.11.md`
  written (rc.9 convention: RC + date + intended tag `v1.0.0-rc.11`
  NOT created + baseline `ea9d7e7`). Item-5 staleness (FTS "NON activé")
  flagged again — architect owns the correction, not applied.
- Q9: fresh exe copied to `release-v1.0.0/` (replacing rc.10-era bytes);
  stage-rc.11 held exactly the 9 DELIVERY-SET files. Q10:
  `Compress-Archive` → `Baraka-UltraPOS-v1.0.0-rc.11-Delivery.zip`
  (**223,089,610 bytes**); zip manifest verified = exactly the 9 files;
  stage dir removed (NO_LEFTOVERS).
- Q11: `sha256sum -c SHA256SUMS.txt` → **all 18 lines OK**; `ls -1`
  unlisted set = exactly the 6 historical records + SHA256SUMS.txt itself
  (no rc.3 — recorded gap, not an error). SHA256SUMS.txt own hash:
  `94ad9792cf411d286bce805e282062d77b5098d222ce806e5c6adaa7167404e6`.
- Q12 seal: status shows ONLY the packaging footprint —
  `M release-v1.0.0/…setup.exe`, `M SHA256SUMS.txt`,
  `?? audit/REL-010/`, `??` 4 letter copies + release notes.
  (The rc.11 zip is gitignored by rule `.gitignore:56`, like all prior zips.)
- Full SHA-256 list: see `release-v1.0.0/SHA256SUMS.txt` (18 lines:
  exe `532c8e0f…359d5a`, rc.11 zip `3f945137…028d27f8`, notes/gates/guides
  as listed; historical zips unchanged).
- Smoke: S1–S4 NOT-EXECUTED (no test machine/GUI in this session — an
  interactive install cannot be driven from here; migration coverage stands
  on G3 + replay/restore drills, cited as evidence not substitute).
  S5 owner hardware steps PENDING-OWNER. S6 EXECUTED green (zip manifest
  exact 9/9 + all hashes OK on the delivery files).
- STATE done (delivery complete, blocked-network cleared by owner seed).
  DONE: bundle + 9-file set + full hashes + REPORT. REMAINING: owner trial
  (install, S1–S5 on hardware, letter item-5 correction by architect).
  No commit, no tag. NEXT READY: YES.
