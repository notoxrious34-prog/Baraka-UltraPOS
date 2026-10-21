# TASK-AUD-037 REPORT — rc.10 delivery artifact integrity (read-only)

HEAD `d2a303f` confirmed. No file over 50 MB was re-hashed (222 MB exes/zips
recorded by stored sum + mtime only). No builds, no DB access, probe untouched.

## 1. Artifact table (`release-v1.0.0/`)

| name | bytes | mtime | stored sum? | recomputed? | verdict |
|---|---|---|---|---|---|
| `Baraka UltraPOS_1.0.0_x64-setup.exe` | 222308475 | Oct 17 15:45 | yes (`SHA256SUMS.txt`: `5512cb43…0df5d`) | NO (>50 MB) | STORED-ONLY |
| `Baraka-UltraPOS-v1.0.0-rc.10-Delivery.zip` | 222342588 | Oct 17 15:46 | yes (`SHA256SUMS.txt`: `06a020d5…73b4b0`) | NO (>50 MB) | STORED-ONLY |
| `Baraka-UltraPOS-v1.0.0-rc.2-Delivery.zip` | 221605801 | Oct 12 11:25 | NO | NO (>50 MB) | MISSING-SUM |
| `Baraka-UltraPOS-v1.0.0-rc.4-Delivery.zip` | 221627920 | Oct 12 21:16 | NO | NO (>50 MB) | MISSING-SUM |
| `Baraka-UltraPOS-v1.0.0-rc.5-Delivery.zip` | 222033234 | Oct 15 10:26 | NO (rc.5 sum covers exe only) | NO (>50 MB) | MISSING-SUM |
| `Baraka-UltraPOS-v1.0.0-rc.6-Delivery.zip` | 222259395 | Oct 15 12:19 | NO (rc.6 sum covers exe only) | NO (>50 MB) | MISSING-SUM |
| `Baraka-UltraPOS-v1.0.0-rc.7-Delivery.zip` | 222268828 | Oct 15 17:20 | NO | NO (>50 MB) | MISSING-SUM |
| `Baraka-UltraPOS-v1.0.0-rc.8-Delivery.zip` | 222344143 | Oct 16 21:12 | NO | NO (>50 MB) | MISSING-SUM |
| `Baraka-UltraPOS-v1.0.0-rc.9-Delivery.zip` | 222344137 | Oct 17 12:03 | NO | NO (>50 MB) | MISSING-SUM |
| `GUIDE-UTILISATION.html` | 33825 | Oct 17 15:46 | n/a (doc) | `b2faabf6…ffad` | HASHED |
| `LISEZMOI-README.txt` | 2036 | Oct 12 10:56 | n/a (doc) | `6af79fcb…94db98` | HASHED |
| `QUESTIONNAIRE-COMPTABLE-FR.html` | 32995 | Oct 15 16:43 | n/a (doc) | `6c7c655e…650164` | HASHED |
| `RELEASE-NOTES-v1.0.0-rc.7.md` | 2779 | Oct 15 16:43 | n/a | `b4b38e04…6882b91` | HASHED |
| `RELEASE-NOTES-v1.0.0-rc.8.md` | 2471 | Oct 16 21:11 | n/a | `ae966aec…47832ad27c` | HASHED |
| `RELEASE-NOTES-v1.0.0-rc.9.md` | 2442 | Oct 17 12:03 | n/a | `25e0d533…f14661c16` | HASHED |
| `SHA256SUMS.txt` (210 B) | 210 | Oct 17 15:46 | self | `73276568…f65d7` | HASHED |
| `SHA256SUMS-rc.5.txt` (102 B) | 102 | Oct 15 10:26 | self | `3d764653…9f44` | HASHED |
| `SHA256SUMS-rc.6.txt` (102 B) | 102 | Oct 15 12:19 | self | `dc62d363…84ffeb` | HASHED |

(`SHA256SUMS.txt` content: exe `5512cb43…0df5d`, rc.10 zip `06a020d5…73b4b0`.
`SHA256SUMS-rc.{5,6}.txt` each cover that rc's setup exe only:
`40d48ff0…6a5a8e46b39` / `80c1ba34…43844a70` — the rc.5/rc.6 zips themselves
are uncovered. Full recomputed hashes in §1 rows above.)

## 2. Coverage gaps (explicit)
1. rc.3 delivery zip entirely absent (rc.2 → rc.4 jump); no rc.3 sums or notes.
2. rc.2/rc.4/rc.7/rc.8/rc.9 zips have NO stored sums anywhere.
3. rc.5/rc.6 sums cover the setup exe only, not their delivery zips.
4. No `RELEASE-NOTES-v1.0.0-rc.10.md` (notes stop at rc.9); no rc.10-specific LISEZMOI delta recorded.
5. `Baraka UltraPOS_1.0.0_x64-setup.exe` (no rc tag) is the presumed rc.10 installer, covered by `SHA256SUMS.txt` — naming ambiguity vs the rc-tagged zips.

## 3. rc.11 delivery set (must contain)
Setup exe + delivery zip + `SHA256SUMS.txt` covering **all** shipped zips/exe
(no exe-only partial sums) + `RELEASE-NOTES-v1.0.0-rc.11.md` + `LISEZMOI-README.txt`
+ `GUIDE-UTILISATION.html` + `QUESTIONNAIRE-COMPTABLE-FR.html`. All templates
confirmed present in `release-v1.0.0/`.

## 4. Probe state (one-liner, untouched)
Background 80k backfill probe still running (log shows `running 1 test`, no
completion markers); not polled, not killed, no cargo/npm started by this task.
