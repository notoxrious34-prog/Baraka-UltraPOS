# Local `.sig` signing + one-click updater — runbook (readiness wave)

**Ownership:** the owner executes this on the release machine. The private
key NEVER enters the repository, the chat, or any screenshot. The
assistant prepares everything up to the key step.

## 1. One-time key setup (owner only)

```powershell
# Generate (Tauri CLI wraps minisign):
npx tauri signer generate -w "$env:USERPROFILE\.tauri\baraka.key"
# Contents: private key file. Back it up OFF the repo (encrypted USB).
# Public key printed by the command goes into
# src-tauri/tauri.conf.json → plugins.updater.pubkey
# (replace the current placeholder value).
```

Current state: `bundle.targets=["nsis"]`, `createUpdaterArtifacts=true`,
endpoint
`https://github.com/notoxrious34-prog/Baraka-UltraPOS/releases/latest/download/latest.json`,
updater dialog off (all UX lives in Settings → Update Center).

## 2. Release bundle with signatures (owner only)

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.tauri\baraka.key" -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""   # set only if the key has one
npm run build
npx tauri build -j 2
```

Expected artifacts (`src-tauri/target/release/bundle/`):

- `nsis/Baraka UltraPOS_<version>_x64-setup.exe`
- `nsis/Baraka UltraPOS_<version>_x64-setup.exe.sig` ← must exist
- `latest.json` ← upload alongside the installer

`G8-EXIT:1 "no private key"` before this step is environmental, not a code
defect: the build is complete; only the signature is missing.

## 3. Publish (owner only)

1. Create the GitHub release `vX.Y.Z`, upload the `.exe`, the `.exe.sig`,
   and `latest.json` (exact file names — the updater endpoint reads
   `latest.json` from `releases/latest/download/`).
2. On the shop PC: Settings → Update Center → Check → Install. The updater
   verifies the minisign signature against `tauri.conf.json` pubkey before
   touching the install (signature-checked, dialog-free).
3. Trial-installer evidence chain for REL-063: installer size + SHA-256 in
   the release notes, plus one screenshot of the Update Center success
   state on the shop PC.

## 4. Rotation

New key → new pubkey in `tauri.conf.json` → commit → rebuild → publish.
Old installers keep verifying against the pubkey they shipped with; the
chain never breaks as long as each release carries its own `.sig`.
