# ADR-074 - Destructive-action confirmation pattern
Status: ACCEPTED (SESSION-ARC-001)
Context: Restore requires typing the English word RESTORE. That check exists only in the frontend (src/stores/backup-modal-store.ts:71), not in Rust (AUD-004 S6d). The owner's keyboard layout is French and the UI is bilingual.
Decision: Typed English keywords are prohibited as confirmation. Irreversible actions (restore, and any similar action introduced later) use hold-to-confirm: press and hold the confirm button for at least 1.5 s with visible progress; the keyboard equivalent is holding Enter or Space. A plain-language summary of consequences is shown in the active locale before the button is enabled. The server-side precheck, quarantine and rollback protocol of ADR-062 is unchanged and remains authoritative.
Consequences: The restore dialog is redesigned in a DSN task; engine restore logic is not modified.
Validation gate: none.
