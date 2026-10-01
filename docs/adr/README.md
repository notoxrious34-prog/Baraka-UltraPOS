# Architecture Decision Records (ADR)

Project: Baraka UltraPOS. Maintained by the Architect (Charter v2.0).

## Rules
- One file per ADR from ADR-066 onward: docs/adr/ADR-NNN-short-title.md.
- ADR-001..065 are indexed in INDEX.md. Their definitions live elsewhere (see INDEX.md); ADR-016..061 have no recoverable definition document.
- An accepted ADR is never rewritten. Only its Status line may change. A changed decision requires a new ADR that supersedes it.
- Engine invariants (Charter section 5) cannot change without an ADR.
- Statuses: PROPOSED, ACCEPTED, SUPERSEDED BY ADR-NNN, REJECTED.
- Owner validation gates stay explicit in each ADR. An ADR never claims legal, tax, or hardware compliance.
- Next free id: ADR-075.

## Template
    # ADR-NNN - Title
    Status: PROPOSED | ACCEPTED | SUPERSEDED BY ADR-NNN
    Context: facts and evidence (file:line, audit id).
    Decision: what is decided, in imperative form.
    Consequences: what changes, what is deferred.
    Validation gate: open owner or professional gates, if any.
