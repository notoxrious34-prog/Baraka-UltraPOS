# Architecture Decision Records (ADR)

Project: Baraka UltraPOS. Maintained by the Architect (Charter v2.0).

## Rules
- One file per ADR from ADR-066 onward: docs/adr/ADR-NNN-short-title.md.
- ADR-001..065 are indexed in INDEX.md. Their definitions live elsewhere (see INDEX.md); ADR-016..061 have no recoverable definition document.
- An accepted ADR is never rewritten. Only its Status line may change. A changed decision requires a new ADR that supersedes it.
- Engine invariants (Charter section 5) cannot change without an ADR.
- Statuses: PROPOSED, ACCEPTED, SUPERSEDED BY ADR-NNN, REJECTED.
- Owner validation gates stay explicit in each ADR. An ADR never claims legal, tax, or hardware compliance.
- Next free id: ADR-091.
- ADR-089: Data-Retention Invariants, In-Place Upgrade Safety, and Downgrade Policy — ACCEPTED (docs/adr/ADR-089-data-retention-and-upgrade-preservation.md; drill record docs/operations/DRILL-DATA-RETENTION-UPGRADE-v1.0.0.md).
- ADR-090: Data Exchange Hub, POS Fast-Sale Optimization, and Responsive Catalogue Layout — ACCEPTED (docs/adr/ADR-090-data-exchange-hub-and-cockpit-optimization.md; forensic record docs/operations/FORENSIC-AUDIT-REPORT-v1.0.0.md).

## Template
    # ADR-NNN - Title
    Status: PROPOSED | ACCEPTED | SUPERSEDED BY ADR-NNN
    Context: facts and evidence (file:line, audit id).
    Decision: what is decided, in imperative form.
    Consequences: what changes, what is deferred.
    Validation gate: open owner or professional gates, if any.
