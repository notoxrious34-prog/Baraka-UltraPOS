# Baraka Stock Skill
Use for products, units, warehouses, inventory, kits, and stock movements.

- Store stock in the base unit using fixed-point integers.
- Every conversion is exact and deterministic.
- Prevent negative stock unless explicitly authorized by policy.
- Stock levels are operational state; stock movements preserve the audit history.
- Kits decrement component stock atomically.
