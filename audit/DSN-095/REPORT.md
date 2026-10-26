# TASK-DSN-095-R REPORT — Backup/restore/reset screen redesign (block lifted by BRK-094 `bc11c8c`)

STATUS: **DONE**. DO NOT COMMIT.

Prior trace committed as `e581cdf` (engine-side 0 B verdict); engine fix
landed as `bc11c8c` (0017 + real size/trigger). This report replaces the
earlier version in full.

## 1. FAIL-before (owner-verdict "chaos", quoted from the pre-redesign tree)

- Raw UTC timestamps: `BackupSettingsTab.tsx:162`
  (`<bdi dir="ltr">{row.createdAt}</bdi>` — RFC3339 dump, no locale).
- "0 B" sizes: `:172` (`formatBytes(row.fileSizeBytes)` fed literal `0`
  from `backup.rs:175`).
- Truncated raw checksums as the status column: `:175`
  (`checksumPrefix(row.sha256Checksum)` — 12 hex chars where a VERIFIED
  badge belongs).
- Duplicated actions: tab create (`:90-99`) + text refresh (`:100-109`),
  plus the danger-zone verified-backup entry
  (`FactoryResetSection.tsx:188`).
- Type pill showing the hardcoded wire code (`:168`
  `<bdi dir="ltr">{row.triggerType}</bdi>`, always MANUAL).
- Raw-code status: no localized labels anywhere on the tab.

## 2. 0 B verdict + wire lines — ENGINE, now FIXED upstream

`backup.rs:175 file_size_bytes: 0` + `:177 Manual` hardcodes removed by
BRK-094; rows now carry measured size + trigger (`manual|scheduled|
cash_close|reset`, migration 0017, legacy NULL → UI "—").
Wire re-verified this task (`src/lib/ipc.ts:2458-2468`):
`BackupMetadata.fileSizeBytes: number | null`, `triggerType`, `status`,
`sha256Checksum`, `createdAt` — all present.
One stale facade type found and fixed (no hardcoding):
`BackupMetadataWire.file_size_bytes: number` → `number | null`
(`src/lib/ipc.ts:2473`) so legacy NULL rows typecheck through.

## 3. Component lines (presentation only, zero engine)

- `src/features/settings/settings-helpers.ts` — `formatBytes(bytes,
  locale)`: FR `o/Ko/Mo/Go` (fr-DZ numerals), AR `ب/ك.ب/م.ب/غ.ب`
  (ar-DZ numerals); NULL/≤0/NaN → "—", never 0. New
  `formatBackupDate(iso, locale)` (medium date + short time, raw fallback
  never blank).
- `src/features/settings/BackupSettingsTab.tsx` — card rows (filename +
  localized status pill; locale date · size + localized trigger pill;
  VERIFIED `backups.verifiedOk` badge on SUCCESS, failure line otherwise;
  `<details>` checksum affordance); section heading + subtitle; single
  primary icon+text create (`backups.createNow`/`creating`) + single icon
  refresh (`backups.refresh` title + aria-label); `backups.empty /
  loading / retentionNotice / successCreated{file}`; restriction view
  unchanged.
- `src/features/settings/FactoryResetSection.tsx` — calm danger card
  (thin border, `DangerIcon` SVG — the old `⚠` text glyph is gone);
  preconditions as quiet `PrecheckRow` checklist (glyph + text, no loud
  pills); newest-backup line carries the localized status; token confirm,
  execute, refresh, error and outcome flows byte-identical in behavior.
- `src/lib/i18n.ts` — `backups.triggerReset` (FR "Réinitialisation" / AR
  "إعادة ضبط"), `backups.checksumDetails` (FR "Détails SHA-256" / AR
  "تفاصيل SHA-256"), both locales.
- `src/components/pos/modals/BackupModal.tsx` — `triggerKey` gains the
  `RESET` case (RESET rows previously fell through to "Manuelle").
- `src/lib/ipc.ts` — wire `file_size_bytes: number | null` (see §2).

## 4. Pin lines (`settings-module.test.ts`, 30/30 in file)

- "renders locale rows" — 17 `backups.*` keys in source,
  `formatBackupDate(row.createdAt, locale)`,
  `formatBytes(row.fileSizeBytes, locale)`, `<details`, `<bdi dir=`,
  negatives: no `{row.createdAt}</bdi>`, no `{row.triggerType}</bdi>`,
  no `settings.colFile`.
- "exactly one primary create + one refresh" — single
  `await createBackup(sessionToken, "MANUAL")` site; two
  `t(locale, "backups.refresh")` hits (title + aria-label of the one icon
  button). Danger-zone keeps its own entry (`settings.resetCreateBackup`
  + scoped `settings.refresh` in `FactoryResetSection.tsx`).
- "locale sizes" — `512→"512 o"`, `2048→"2 Ko"`, `5MiB→"5 Mo"`,
  null/0/-1→"—"; AR contains `ب`, `م.ب`, null→"—".
- "locale dates" — FR/AR contain year, no ISO dump; garbage in → raw out.
- "key symmetry" — `triggerReset`, `checksumDetails`, `verifiedOk`,
  `retentionNotice` each exactly 2× in `i18n.ts` + FR/AR copy spot-checks.

## 5. PNGs + asserts (6/6 distinct SHAs, 18/18 asserts, 0 console)

Harness: headless Chrome CDP vs `vite :5173`, `__TAURI_INTERNALS__`
stubbed with canned rows (incl. a legacy NULL-size SCHEDULED row and a
FAILED row); real login form → settings → backups tab; FR + AR.
`audit/DSN-095/capture-results.json`: **18/18 PASS, consoleErrors 0**
(danger close-up re-shoot: 0 console errors on its own run).
Vision-QA: all six viewed (list-fr, list-ar, restoring-fr, restoring-ar,
danger-fr, danger-ar — "double __" in two readbacks is a small-mono-font
misread, stub bytes are single-underscore, verified by grep).

| PNG | SHA-256 (short) | Assert coverage |
| --- | --------------- | --------------- |
| `backup-list-fr-light.png` | `8af6afdb…` | 2,5 Mo · Manuelle/Planifiée/Clôture · Réussie/Échouée · "Intègre (SHA-256 vérifié)" · Détails SHA-256 · no UTC · no h-scroll |
| `backup-list-ar-light.png` | `af1b9cd8…` | يدوية · ناجحة · "—" · `[dir=rtl]` shell · no raw codes |
| `backup-restoring-fr-light.png` | `e8a598ff…` | modal opens from Opérations · report 8/8 checks · `Confirmation obligatoire` gate |
| `backup-restoring-ar-light.png` | `8a99d171…` | `تأكيد إلزامي` · 8/8 checks AR · `اكتب RESTORE` · RTL modal |
| `backup-danger-fr-light.png` | `72e85d50…` | element-clipped card · calm checklist 3×✓ · token empty → execute disabled |
| `backup-danger-ar-light.png` | `ec4ab524…` | element-clipped card · `الشروط المسبقة` 3×✓ · RTL mirroring · no defects |

All six SHAs pairwise distinct (danger shots are element clips, not
reused list frames).

## 6. Gates

| Gate | Result |
| ---- | ------ |
| G4 `npm run lint` | exit 0, GREEN |
| G5 `npx tsc --noEmit` | clean, GREEN |
| G6 `npm run build` | ✓ built in 4.89s, GREEN |
| G7 `npx vitest run` | **81 files / 822 tests passed**, GREEN |
| G1–G3 | N/A — zero Rust touched (state reason) |

No product file changed since the green run — tallies stand as stated.

## 7. NO_LEFTOVERS + ports

- Scratch deleted: `dsn095-capture.mjs`, `dsn095-danger.mjs`,
  `dsn095-vite.log`, `dsn095-chrome/` — `ls scratch | grep dsn095`
  empty (proof below).
- No port left listening: vite :5173 + CDP :9222 servers killed (own
  PIDs only, verified by `curl` DOWN + port check).
- Ad-hoc scripts: none remain in-repo (`git status` shows only the
  capture PNGs + `capture-results.json` under `audit/DSN-095/`).

NEXT READY: YES.
