# CTP-SESSION-ARC-035 — Silent-printing pivot: preamble → spooler proof → E-matrix → TEXT rejection → probe trials

Status: session sealed (R1–R10). Range: `818f192` (BRK-099) → `574f363` (REL-016). Prior package: CTP-SESSION-ARC-034 (incoming, up to `85db155`).

## 1. Commits (8, tree clean throughout)

| Commit | Content | Evidence |
|---|---|---|
| `818f192` | BRK-099 TSPL preamble (SIZE→GAP→DENSITY→SPEED→CODEPAGE→DIRECTION→CLS) | 576/0, OWNER-VERIFY gap 3 |
| `0c53c92` | REL-014 trial 4 + owner script §5 | installer 223,298,532 B, `050a42b5…80122d` |
| `b8139ba` | AUD-046 spooler completion (COMPLETES-PROVEN, no GetJob anywhere, Seagull INFERENCE) | 53-line REPORT |
| `e033fdb` | AUD-047 TEXT rejection + BITMAP design (CODEPAGE name-form most suspicious; 6.4KB/52.5KiB) | 80-line REPORT |
| `051d2d9` | BRK-100 probe switch (normal/no_codepage/ascii_only/font1_only, TEMPORARY-DIAGNOSTIC) | 582/0, +402/-26 |
| `858dbaa` | REL-015 trial 5 (no selector — proved untestable) | 223,305,302 B, `9a35bb9b…` |
| `9ffc7d2` | DSN-100 selector UI (8/8 pins, 2 shots) | 882/0 |
| `574f363` | REL-016 trial 6 with 8-cell script | 223,306,717 B, `0aa4ae51…` |

## 2. Decisive evidence chain

- Owner trial 4: preamble alone does NOT unblock (Get still needed per print).
- `.inf` (Seagull V3 TSC, DriverVer 11.5.0.0): driver stack proven, not raw passthrough.
- Owner 4-answers + queue shots: jobs BOTH park-blank AND vanish-completed with zero LED.
- E-matrix: E1✓ E2✗ E3✓ E4✗ → LANGUAGE verdict (transport/suspend/wake/NVRAM all DEAD).
- Get reinterpretation (INFERENCE): Tool USB session resets latched parser / selects emulation.

## 3. Standing rules (unchanged, all honored)

Live DB never opened; no new deps; `hermes verify` banned (attempted once in BRK-099, killed, VOID);
foreign PID 14996 never touched; installers gitignored; DO NOT COMMIT honored (architect commits);
NO_LEFTOVERS; one task/reply.

## 4. Open

Owner P1–P3 8 cells (trial 6) → winner lock-in (remove TEMPORARY switch) or BITMAP pivot
(AUD-047 design in hand). Preamble gap default 3 stays OWNER-VERIFY; Arabic verdict pending winning mode.
