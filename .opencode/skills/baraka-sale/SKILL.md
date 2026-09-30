# Baraka Atomic Sale Skill
Use for the sale engine and checkout transaction.

Canonical sequence:
Idempotency → BEGIN IMMEDIATE → validation → numbering → document/items → stock guard/decrement → movements → credit/payment → cash → audit → print_jobs → COMMIT → physical printing.

Retry only classified transient database errors. Never retry deterministic business failures.
