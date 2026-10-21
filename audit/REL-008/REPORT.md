# TASK-REL-008 REPORT — pnpm-lock.yaml reconciled to @tauri-apps/api 2.12.4

## Step-0 versions (baseline)

| Source | Value |
|--------|-------|
| `package.json:17` specifier | `^2.12.4` (unchanged, not edited) |
| `node_modules/@tauri-apps/api` installed | `2.12.4` |
| Committed `pnpm-lock.yaml` resolved | `2.11.1` → INCONSISTENT (confirmed) |
| `git status -- pnpm-lock.yaml package.json` baseline | clean (no output) |
| HEAD observed | `d13d79f` (task context states `0bc4e99` — superseded by landed commits; procedure unaffected, facts re-verified above, not assumed) |

## Authorized command (single PM invocation — no other ran)

```
pnpm install --lockfile-only --prefer-offline
```
Exit 0, ~3s, offline metadata sufficient (`downloaded 0`), no approve-builds,
no add/update. No second PM command invoked.

## Lockfile diff (ACCEPTED — tauri-api-only)

`git diff --stat -- pnpm-lock.yaml` → `1 file changed, 5 insertions(+), 5 deletions(-)`:
- Hunk 1 (`importers:`): specifier `^2.1.1`→`^2.12.4`, version `2.11.1`→`2.12.4`.
- Hunk 2 (`packages:`): resolution integrity hash updated.
- Hunk 3 (`snapshots:`): snapshot key `@tauri-apps/api@2.11.1`→`@2.12.4`.
- No other package drift (every `+/-` line inside these 3 hunks; verified via
  hunk headers). Lockfile left uncommitted for Architect review (no `git add`).

## Smoke (installed modules, unchanged)

- G4 `npm run lint` → exit 0 (`--max-warnings 0`).
- G6 `npm run build` → `✓ built in 5.43s`.

## STATE / DONE / REMAINING

STATE done. DONE: lockfile resolves exactly the installed 2.12.4, diff
scoped, smoke green, REPORT written. REMAINING: Architect review + commit
(not mine). NO COMMIT made. NEXT READY: YES.
