# UPD-001 — Update Center (GitHub Releases auto-update) — REPORT

Owner order: leave the printer path parked, build professional auto-update
through `notoxrious34-prog/Baraka-UltraPOS`, Update Center inside Settings,
100% by the architect, deep review of all files first, visual design critical.

## 1. What was reviewed before writing code

- `src-tauri/src/lib.rs` (plugin registration seam), `src-tauri/Cargo.toml`,
  `src-tauri/tauri.conf.json` (bundle truth), `package.json` (`@tauri-apps/api 2.12.4`)
- `src/features/settings/SettingsModule.tsx` (tab registry, permission gates),
  `BackupSettingsTab.tsx` (visual language, backup facade),
  `src/lib/ipc.ts` (`backups_create`), `src/lib/i18n.ts` (FR/AR dict shape),
  `src/App.tsx` (startup effects)
- Diagnosis: no remote, no `.github/`, no updater, Tauri 2 + NSIS single-channel.

## 2. What was built

- Engine: `tauri-plugin-updater = "2"` + `@tauri-apps/plugin-updater@^2`
  (the two official updater deps ONLY — `plugin-process` evaluated and REMOVED,
  Windows `install()` exits + relaunches by itself, no extra dep).
- `tauri.conf.json`: `createUpdaterArtifacts: true`,
  `plugins.updater = { active, dialog: false, endpoints: [latest.json], pubkey }`.
- `.github/workflows/release.yml`: tag `vX.Y.Z` → signed Windows NSIS +
  `latest.json` (secrets owner-managed, never committed).
- Frontend: `src/lib/update-client.ts` (UpdateFlow: probe → download →
  backup FIRST → install; OFFLINE stable code), `src/stores/update-center-store.ts`
  (single-flight, silent auto-probe), `src/features/settings/UpdateCenterTab.tsx`
  (status / mode / release / security cards), wired in `SettingsModule` under
  System with `SYSTEM_SETTINGS_MANAGE`, silent probe in `App.tsx`.
- Gates enforced in UI: install requires CLOSED cash session + admin, backup
  verified before installer, signature mandatory, no downgrade, no popups ever.

## 3. Pins (all green, zero network)

- `src/lib/update-client.test.ts` — 8/8 (probe mapping, progress accumulation,
  backup-before-install order, no-install-on-backup-fail, stable no-handle codes,
  offline mapping, fingerprint pin).
- `src/features/settings/update-center.test.tsx` — 6/6 wiring pins.
- G1 `cargo fmt --check` EXIT 0; G2 `cargo clippy --all-targets --all-features -- -D warnings` EXIT 0;
  G3 `cargo test --workspace -j 2` EXIT 0 — 71 suites / 587 passed / 0 failed
  (identical to pre-change baseline: no regressions);
  G4 lint EXIT 0; G5 typecheck EXIT 0; G7 `npx vitest run` 903 passed / 0 failed.

## 4. Visual evidence

- `audit/UPD-001/update-center-fr-light.png` (FR, fluid width): 3/3 mode/check
  buttons fully visible, no horizontal overflow (`scrollWidth == clientWidth`
  asserted in-page), `Télécharger` 100% + security card verified second shot.
- `audit/UPD-001/update-center-ar.png` (AR): RTL PASS — right-aligned, bidi
  numbers LTR, colon/paren mirroring correct, `النسخة: 1.1.0` fully readable
  after the `dir="ltr"` → split-span bidi fix, no clipping, no typo.
- Capture scaffolding (`__upd/`, `vite.__upd.config.ts`) deleted (NO_LEFTOVERS),
  capture vite server killed, no port left listening.

## 5. G6 bundle — trial-9 (SEALED)

- Frontend `npm run build`: EXIT 0 (`tsc && vite build`, built in ~7s).
- Rust release: `Finished release [optimized] in 12m 44s`,
  `target/release/ultrapos.exe` (20,932,608 B).
- NSIS: `Baraka UltraPOS_1.0.0_x64-setup.exe` — 224,397,270 B —
  SHA-256 `df26307c68278378ed52400d6ee8735943a4a7989ab0ee7138cfbe4e6903d554`
  (≠ trial-8 `173594db…c8635b`; +1.1 MB = updater reqwest/hyper tree).
- Signing gate (BY DESIGN, not a failure): `tauri build` stops at updater
  artifacts — `TAURI_SIGNING_PRIVATE_KEY` lives only in the owner's GitHub
  Secrets, so no `.nsis.zip`/`.sig` on this machine. The setup.exe installs
  and runs (updater stays inert until the first signed CI tag emits
  `latest.json` + artifacts).
- Smoke: own child `ultrapos.exe` PID 18468 booted, `SMOKE_EXIT:0`, no
  surviving process; live DB metadata untouched (`baraka.db` 774,144 B,
  mtime Oct 31; `-wal` identical; only `-shm` mtime moved — same artifact
  as all prior trials). No port left listening (5199 killed + verified).

## 6. Owner trial-9 script (setup.exe above)

1. Install over trial-8, open Settings → Updates & Maintenance.
2. Expect: version `1.0.0`, last-check stamp, mode auto/manual, release card
   appears only when a signed `vX.Y.Z` release exists (none yet → quiet idle).
3. Offline machine: no popup, status stays readable — the shop never waits.
4. To go live: link remote → Secrets(`TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]`)
   → push tag `v1.1.0` → CI publishes signed release → clients see 1.1.0.

## 7. Explicitly NOT done (owner-owned)

- `git remote` not linked, `TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]` not in
  Secrets, no tag pushed — remote + secrets are_owner-only by standing rule.
- Commit below seals the sealed tree (single commit, UPD-001).

STATE: DONE / REMAINING: owner trial-9 verdict + remote/Secrets/tag (owner-only). NEXT READY: YES.
