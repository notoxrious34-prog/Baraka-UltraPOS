# ADR-103 — Confined `unsafe` Exception for the Windows Spooler FFI Crate

Status: ACCEPTED. Date: 2026-10-24 (SESSION-ARC-028). Authority: Architect (Charter v2.0 §1.2).

## Context

Owner mandate: USB-direct printing (no COM bridge). Design: RAW bytes to the
named Windows print queue via Win32 spooler API (OpenPrinterW /
StartDocPrinterW / WritePrinter / EndDocPrinter / ClosePrinter). All five calls
are `pub unsafe fn` in the `windows` crate (0.62.2, verified in local cargo
cache). Every existing manifest forbids `unsafe_code`
(`application`, `commands`, `domain`, `infrastructure`, `src-tauri`).
Evidence: audit/BRK-089/REPORT.md (STATE BLOCKED).

## Decision

One new crate, `crates/spooler_raw`, is the ONLY place where `unsafe` may
appear, under these confinement rules:

1. The crate exposes a 100% safe API (`open_queue(name)`, `write_raw`,
   RAII close; no `pub unsafe fn`, no raw handles escaping).
2. Every `unsafe` block carries a `// SAFETY:` comment naming the upheld
   invariant (valid handle, wide-string NUL-termination, buffer lifetime).
3. The crate does NOT inherit any workspace forbids that would conflict;
   it sets its own `[lints]` explicitly (deny warnings, forbid nothing it
   cannot uphold — recorded in its manifest).
4. No other crate may add `unsafe`: enforced by keeping `unsafe_code =
   "forbid"` in all five existing manifests plus a gate grep
   (`grep -rn "unsafe" crates/*/src` must return only `spooler_raw`).
5. `windows = "0.62"` is the only new dependency (authorized in
   TASK-BRK-089; reaffirmed here).

## Consequences

- Win32 unsafety is confined to ~100 lines, reviewable in isolation.
- The forbid rule stays at full strength everywhere else.
- If a second FFI need ever arises, it goes through this same crate or a
  new ADR — never a lint downgrade.
