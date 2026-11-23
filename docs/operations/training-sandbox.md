# Training Sandbox — runbook (TASK-BRK-131)

**Rule zero: the sandbox NEVER touches live data.** The live till database
lives under the OS app-data directory (`%APPDATA%/com.baraka.ultrapos`).
Training runs on a separate directory selected through the
`BARAKA_DATA_DIR` environment override. No override → live mode, no banner.

## 1. Launch a training session

```powershell
$env:BARAKA_DATA_DIR = "D:\Baraka-Training"
.\ultrapos.exe
```

- First launch creates a fresh database in `D:\Baraka-Training` (migrations
  run automatically) and shows the first-run setup: create a TRAINING admin
  (e.g. `formateur`), never reuse a live username.
- Every screen carries an amber strip:
  FR `MODE FORMATION — base d'entraînement, pas la caisse réelle` /
  AR `وضع التدريب — قاعدة تدريبية وليست الصندوق الحقيقي`, plus the
  terminal id. **No strip = live till. Stop and check the environment.**
- The window title also shows `[BARAKA_DATA_DIR]`.

## 2. Suggested 30-minute drill

1. Catalog: create 3 training products (or one matrix model + `Série`).
2. POS: cash sale, credit sale (`Ancien Solde + Document − Versement =
   Nouveau Solde`), one return, one exchange with `PIN Gérant`.
3. Cash: open → movements → close (a `SHIFT_CLOSE` backup is captured
   automatically; failures warn, never roll back the close).
4. Reports: daily sales, credit outstanding, signal cockpit.
5. Settings → Backups: verify the scheduled-capture status and the 48h
   staleness badge (red when no SUCCESS backup inside 48h).

## 3. Reset between trainees

Close the app, delete the sandbox directory contents, relaunch — the
first-run setup reappears. Never delete or copy anything under the live
`%APPDATA%/com.baraka.ultrapos` directory.

## 4. Technical notes

- Source of truth: `app_environment.sandbox` (Rust `data_dir_override`,
  computed once at startup). The banner paints only on explicit `true`;
  browser preview degrades to no banner (fail closed).
- The sandbox database is a full citizen: same 25+ migrations, same
  downgrade guard. A sandbox DB copied onto a live machine is still a
  training DB — verify the banner before any real sale.
- Exception path: `BARAKA_DATA_DIR` invalid → startup aborts with
  `Baraka UltraPOS cannot start: BARAKA_DATA_DIR is invalid (...)`; the
  live directory is never used as fallback.
