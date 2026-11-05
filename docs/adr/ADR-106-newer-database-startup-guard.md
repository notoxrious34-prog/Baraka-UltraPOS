# ADR-106 — Live-startup newer-database guard (downgrade refusal)

Status: ACCEPTED. Date: 2026-11-05 (TASK-BRK-104). Authority: Sovereign (self-tasked under v3.1 §1.3).

## Context

`run_migrations` tolerates unknown future `schema_migrations` rows by design
(pinned by `downgrade_guard_probe_future_version_is_deterministic`:
forward-compatibility by ignore-list, never destroys data). Consequence: a
database last written by a NEWER binary opened silently in an older binary,
which could then misread or corrupt a schema it does not understand. The
restore path already refuses newer schemas (`SchemaNewerThanApp`); the live
startup path had no guard (AUD-049 gap G1).

## Decision

- New pure read-only probe `probe_newer_database` in
  `crates/infrastructure/src/persistence/migration.rs`: refuses with
  `InfrastructureError::DatabaseNewerThanBinary { found, supported }` when
  `MAX(version)` exceeds `supported_schema_version()` (`MIGRATIONS.len()`).
  Missing/empty bookkeeping passes.
- `run_migrations` semantics UNCHANGED (tolerant; all 50+ callers untouched).
- Tauri startup (`src-tauri/src/lib.rs`) calls the probe after migrations
  succeed; a future version aborts launch via the existing `startup_error`
  channel with versions in the message (no raw SQL text).
- Regression law: `newer_database_guard_tests.rs` failed to compile before
  the change (E0432/E0599) and passes after.

## Consequences

- Genuine downgrades now fail fast with a clear message instead of risking
  silent corruption. Upgrades and same-version launches are unaffected.
- Future schema work: bump `MIGRATIONS` only; the horizon tracks automatically.
