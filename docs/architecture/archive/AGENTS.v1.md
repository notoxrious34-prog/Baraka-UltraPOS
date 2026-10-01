# Baraka UltraPOS — Executor Engineering Contract

## Authority

`CONSTITUTION_OF_EXECUTOR.md` is the binding executor constitution and must be loaded in full. Do not summarize, replace, or reinterpret it.

The latest `Baraka UltraPOS — Master Sovereign Architecture & Product Design v1.0.0` is the authoritative technical architecture for this project. It governs later architectural decisions where the older executor constitution contains obsolete implementation details.

When a task touches architecture-critical areas, read the relevant architecture source before modifying code.

## Execution

- Execute exactly one `TASK-BRK-XXX` per session command.
- Never create a second interpretation of the task.
- Never modify SCOPE-OUT.
- Never modify `baraka.db`, `*.db-wal`, or `*.db-shm`.
- Never add dependencies unless the Task Brief explicitly authorizes them.
- Prefer the smallest correct implementation that completely satisfies the acceptance criteria.
- Do not ask the user to type `proceed` to recover from a recoverable tool/provider failure.
- If a provider/tool transport failure cannot be recovered safely, report the exact failure and do not mutate project code as a workaround.

## Architecture Baseline

Runtime data path:

React/TypeScript → Typed Tauri IPC → Rust Commands → Rust Application → Rust Domain → Rust Infrastructure → SQLite/SQLCipher.

Do not introduce Drizzle or Prisma into the runtime database path.

Financial representation:

- Rust Money = i64 centimes newtype.
- Quantity and conversion factor = fixed-point integer newtypes.
- Do not use JavaScript `number` for monetary values.
- IPC monetary wire representation must be canonical decimal strings.

Database integrity:

- SQLite is the local source of truth.
- Financial writes are transactional.
- Ledgers and audit records are append-only.
- Finalized financial documents are immutable.
- Corrections use compensating documents or entries.
- Sale printing uses a transactional print outbox before COMMIT.

## Quality

- TypeScript strict mode.
- Zero `any`.
- Zero `TODO` placeholders.
- Zero `console.log` in production code.
- No silent error swallowing in application/domain code.
- Preserve existing conventions unless the Task Brief explicitly changes them.
- Run only the verification commands required by the Task Brief and the constitution.

## Skills

Use only `baraka-*` Skills and load them on demand. Do not load every Skill automatically.

Use the smallest relevant Skill set for the current task:

- `baraka-financial`
- `baraka-sqlite`
- `baraka-sale`
- `baraka-stock`
- `baraka-ui`
- `baraka-design-excellence`
- `baraka-escpos`
- `baraka-testing`
- `baraka-review`

## Muse Spark Compatibility

Muse Spark 1.3 Contributor via OpenCode Go accepts only `tool_choice: "auto"`. The project-local compatibility plugin sanitizes non-auto values before the request is sent to the Go Responses endpoint.

Do not add `tool_choice` to application code or Task Briefs. This is infrastructure compatibility handling only.

## Usage Discipline

- Keep tool output bounded.
- Read targeted file ranges instead of entire large files when possible.
- Prefer `grep`/`glob` before large reads.
- Do not browse the web unless the Architect explicitly changes the policy.
- Do not launch subagents.
- Do not install packages unless explicitly authorized.
