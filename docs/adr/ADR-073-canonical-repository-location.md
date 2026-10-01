# ADR-073 - Canonical repository location
Status: ACCEPTED (SESSION-ARC-001)
Context: AUD-004 S1 found two copies. D:/Dev/Baraka UltraPOS is a git repository (HEAD 977a804 at audit time). D:\Baraka Project\Baraka UltraPOS has no .git and files dated 2026-09-25 to 2026-09-29.
Decision: D:/Dev/Baraka UltraPOS is the only canonical working tree. The other copy is stale and must never be built, edited, read as a source of truth, or used by any task. Archiving or renaming it requires a future task and the owner's approval; it is never deleted by default.
Consequences: Every task path is under D:/Dev/Baraka UltraPOS.
Validation gate: none.
