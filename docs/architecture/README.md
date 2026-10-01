# docs/architecture - index

| Document | Status |
|---|---|
| EXECUTOR-CONSTITUTION-v2.md | Active. Binding rules for the executor (supersedes v1). |
| archive/CONSTITUTION_OF_EXECUTOR.v1.0.0.md | Archived. Describes an abandoned architecture (ORM, decimal.js, synchronous=NORMAL). Do not follow. |
| archive/AGENTS.v1.md | Archived previous AGENTS.md. Do not follow. |
| CTP-SESSION-ARC-NNN.md | Context Transfer Packages (the architect's memory), added every 10 architect replies. The first one will contain the full Appointment Charter v2.0. |

Pending to be placed here by later tasks: the Master Architecture document v1.0.0, the ADR index (docs/adr/), the design documents (docs/design/).

## Architect documents (SESSION-ARC-001)
- MASTER-ARCHITECTURE-v1.0.0.md - historical architectural baseline (tracked byte-identical).
- MASTER-ARCHITECTURE-ERRATA.md - where reality differs from the baseline, plus carried engineering debts.
- ../adr/INDEX.md - ADR index (001..074); ../adr/README.md - ADR rules.
- Charter v2.0 - imported: CHARTER-v2.0.md into this folder (planned).
- CTP-SESSION-ARC-001.md - Context Transfer Package of architect session SESSION-ARC-001.
- CTP-SESSION-ARC-002.md - Context Transfer Package of architect session SESSION-ARC-002 (successor of SESSION-ARC-001; does not embed the charter).
- BOOT-SESSION-ARC-002.md - generated single-file boot package (CHARTER-v2.0.md + CTP-SESSION-ARC-002.md, byte for byte) via tools/build-boot-package.mjs.
- CTP-COVERAGE-ARC001-vs-ARC002.md - mechanical coverage table: what ARC-001 contains and where each item lives now.
