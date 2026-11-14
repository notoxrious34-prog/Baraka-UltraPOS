# ADR-118: Customer Last-Price Memory (Read-Only Recall)

Status: ACCEPTED (TASK-BRK-111).

## Decision
- New additive table `customer_last_prices` (`0022`): PK
  `(customer_id, product_id, unit_id)`; legs `price_centimes` +
  `discount_per_unit_centimes` + `document_id` + `sold_at`. No backfill:
  empty = unknown, never zero.
- The checkout transaction upserts one row per priced line for named
  customers (step 4b, same atomic tx); guest sales record nothing.
  Latest sale wins (`ON CONFLICT ... DO UPDATE`).
- Recall path: `CustomerRepository::get_last_price` →
  `GetLastPriceUseCase` (`ViewCustomers` gate) → `customers_last_price`
  IPC returning nullable legs. The engine never auto-applies the memory;
  the caller derives `price - discount` (DSN-106 chip).
- Conversion paths (Devis→BL) do not record yet — future work when the
  quote flow exists.

## Consequences
- `restore_tests` simulates pre-22 backups by dropping the table;
  `fts`/`credit`/`replay` horizons bumped to 22.
- RBAC: every selling role reads (Vendeur holds `ViewCustomers`);
  the chip display (DSN-106) decides its own UX gating.
