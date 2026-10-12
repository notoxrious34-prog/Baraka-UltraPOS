# Baraka UltraPOS — Release Procedure (reproducible v1 recipe)

How to rebuild the v1.0.0 release identically, months from now.

## 1. Prerequisites

| Tool | Version in this release | Note |
|---|---|---|
| Node.js | 22.x (V8/JS runtime for the frontend build) | `node --version` |
| Rust toolchain | 1.98.0 (clippy reference in gate output) | `rustc --version` |
| Tauri CLI | ^2.11.5 (devDependency, local) | `npx tauri --version` |
| NSIS | downloaded on first bundle | **internet required ON THE BUILD MACHINE ONLY, first time** |
| WebView2 offline installer | ~127 MB, fetched by the bundler | same asymmetry: build machine only |

Design asymmetry (binding): the build machine may need internet once for
tooling; the TARGET shop machine must need NOTHING — the NSIS bundle embeds
the WebView2 offline installer (`webviewInstallMode: offlineInstaller`).

## 2. Environment

```sh
export CARGO_BUILD_JOBS=2   # MANDATORY on Windows: avoids LNK1102 linker OOM
```

Every cargo invocation that links (build, test, clippy, release build) runs
with this constraint. It is a documented deviation, not a defect.

## 3. Exact commands (in order)

```sh
# 0. Clean checkout at the release commit; confirm version fields:
grep '"version"' package.json src-tauri/tauri.conf.json
grep '^version' Cargo.toml

# 1. Standing gates (must all be green BEFORE bundling):
cargo fmt --all --check                                            # G1
cargo clippy --all-targets --all-features -- -D warnings           # G2
cargo test --workspace -j 2                                        # G3
npm run lint                                                       # G4
npm run typecheck                                                  # G5
npm run build                                                      # G6
npx vitest run                                                     # G7

# 2. Production bundle (records wall time; sets the release binary):
time npm run tauri build                                           # G8

# 3. Artifact fingerprint (the owner's release record):
Get-FileHash 'target/release/bundle/nsis/Baraka UltraPOS_1.0.0_x64-setup.exe' -Algorithm SHA256
ls -la target/release/bundle/nsis/
```

## 4. Verification checklist

- [ ] G1–G7 green with the recorded numbers (G3: 349 passed / 0 failed +
      quoted `17/17 scenarios passed`; G7: 21 files / 180 tests).
- [ ] G8: bundle exits 0 AND the `-setup.exe` exists at the recorded path.
- [ ] `tauri.conf.json`: `targets == ["nsis"]`,
      `windows.webviewInstallMode.type == "offlineInstaller"`,
      NSIS languages French-first. NOTE: the Tauri v2 schema has NO
      `deleteAppDataOnUninstall` key (verified against the CLI schema);
      uninstall data-safety rests on the NSIS default (app data lives under
      `%APPDATA%\com.baraka.ultrapos`, outside the install dir) plus the
      reinstall-over drill evidence — see the audit.
- [ ] `sha256sum Cargo.lock` matches the release record; the lock diff since
      the last task is version-only (workspace members `0.1.0 → 1.0.0`).
- [ ] No `git status` dirt outside SCOPE-IN (this repo is released from an
      uninitialized working tree — see audit §Release record; tag when hosted).

## 5. Tagging (when hosted in git)

```sh
git add -A && git commit -m "Release v1.0.0"
git tag -a v1.0.0 -m "Baraka UltraPOS v1.0.0 production release"
git show v1.0.0 --no-patch   # record commit hash + tag hash in the audit
```

## 6. Data-preservation drill (summary — full protocol in the audit)

1. Clean app shutdown → full copy of `%APPDATA%\com.baraka.ultrapos` aside
   (record file count + sizes) → verify copy.
2. Fresh-install leg: swap the original away, launch, confirm DB recreation
   on the filesystem, shutdown, delete fresh dir, restore original, verify.
3. Upgrade leg: launch over the v13 dir, shutdown, copy, verify the copy via
   `BARAKA_DRILL_DB=<copy> cargo test -p baraka-infrastructure
   --test release_drill -- --ignored --nocapture`.
4. Reinstall-over leg: run the installer again, verify the data dir manifest
   untouched without launching, then launch normally.
5. Downgrade probe: bump the schema marker on a COPY, launch, record the
   behavior as [S], restore the original. Never implement a guard ad hoc.

## 7. Benchmark recording (summary — full protocol in the benchmarks doc)

```sh
CARGO_BUILD_JOBS=2 cargo test --release -p baraka-infrastructure \
  --test release_benchmarks -- --ignored --nocapture
```

Copy the `BENCH …` lines into `docs/operations/release-benchmarks-v1.0.0.md`
with the machine profile table. UI metrics follow the manual protocol in
that document (screen recording at declared fps + working-set sampling).

## 8. Packaging and Production Delivery (v1.0.0-rc.1)

The shop receives a self-contained directory, NOT the `target/` tree:

```text
release-v1.0.0/
  Baraka UltraPOS_1.0.0_x64-setup.exe   # 221 552 107 bytes, on disk only (gitignored)
  SHA256SUMS.txt                        # committed: hash record
  GUIDE-UTILISATION.html                # committed: offline FR/AR handbook
  LISEZMOI-README.txt                   # committed: quick start
```

Reference values (REL-042, re-verified REL-045):

- Size: 221 552 107 bytes (~211 MiB — offline WebView2 bundled).
- SHA-256: `C608C32A8C96B089CB51E187DE473AD0FCCD3E728E8B1C07D0311C1E65D75934`.

Verify a copied bundle before handing it over (PowerShell or certutil):

```powershell
Get-FileHash 'release-v1.0.0\Baraka UltraPOS_1.0.0_x64-setup.exe' -Algorithm SHA256
certutil -hashfile "release-v1.0.0\Baraka UltraPOS_1.0.0_x64-setup.exe" SHA256
```

Compare against `release-v1.0.0/SHA256SUMS.txt`. Field deployment: copy the
whole `release-v1.0.0/` directory to a USB stick (the `.exe` travels on disk;
only the three metadata files live in git). The annotated tag `v1.0.0-rc.1`
marks the exact source state. Operator docs: `docs/operations/HANDOVER-GUIDE.md`
(source) and `release-v1.0.0/GUIDE-UTILISATION.html` (offline rendering).

### Delivery archive verification (REL-049)

The portable zip was extraction-tested into transient
`target/delivery_verify/` (deleted afterward): all 4 files unpacked with
exact sizes (exe 221 552 107, HTML 11 231, readme 2 036, manifest 102
bytes), HTML opens `<!DOCTYPE html>` → closes `</html>`, and the extracted
`.exe` re-hashed to the canonical
`C608C32A8C96B089CB51E187DE473AD0FCCD3E728E8B1C07D0311C1E65D75934`.

Shop-owner integrity check (run next to the downloaded zip):

```powershell
Get-FileHash -Algorithm SHA256 .\Baraka-UltraPOS-v1.0.0-rc.2-Delivery.zip
```

Must print:

```text
16E96A325EF946BF497029B003149110AC62C8B26EA31D86E980BC1D0BF5FA27
```
