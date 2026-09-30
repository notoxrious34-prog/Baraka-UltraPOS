# Baraka Financial Skill
Use for money, tax, discounts, payments, balances, rounding, credit.

- Money is integer centimes in Rust.
- Use i128 for intermediate arithmetic where overflow is possible.
- Rounding is deterministic and explicitly specified by the architecture.
- Never introduce floating-point monetary arithmetic.
- Validate invariants before persistence.
- Snapshot tax, price, discount, and cost data into finalized documents.
- Keep financial operations atomic and idempotent where required.
