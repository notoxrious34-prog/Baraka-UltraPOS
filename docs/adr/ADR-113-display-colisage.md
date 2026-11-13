# ADR-113: Read-Time Colisage Display (Cart, Ticket, A4)

Status: Accepted (TASK-DSN-104, Wave 1).

## Context

BRK-109 froze the Colisage breakdown at read time (`get_items_colisage`,
`DocumentItemDto.colisage`, `null` = predates the engine). The figures
existed on the document wire but no surface showed them; the counter still
read raw base quantities.

## Decision

Display-only, frontend layer, zero engine changes:

- `src/lib/colisage.ts` — pure BigInt mirror of the domain `breakdown`:
  `colisageShares` (ladder from the cart unit snapshots, greedy
  largest-first, `null` on fractions / single-rung noise / invalid
  strings) and `formatColisage` / `formatWireColisage`.
- Cart (`CartTableView`): live sub-caption under the designation
  (`data-testid="cart-colisage"`), computed from the line's own units.
- OS ticket V1+V2 (`graphique-ticket.ts`): `TicketWireItem.colisage` rides
  the document wire through `ticketArgsFromWire` (CheckoutModal and the
  drawer snapshot both pass it verbatim); V1 prints a sub-line in the
  designation cell, V2 a spanning sub-row (`ticket-colisage-row`) — never
  the `qty × price` cell (TICKET-ONELINE agreement holds).
- A4 (`DocumentDetailDrawer`): sub-line under the designation
  (`data-testid="a4-colisage"`, `.a4-colisage` CSS).
- Every surface collapses on `null` / empty — legacy rows print exactly
  as before.

## Deferred (deliberate)

TSPL per-line Colisage text: the engine ticket path is byte-pinned
(80mm truncations, word-wrap mirrors). Appending breakdown text there
changes production bytes and needs its own mirror test + owner sign-off.
Tracked as follow-up, not part of this wave.

## Consequences

- No migration, no IPC change, no arithmetic in the template layer.
- Collapse rule is the anti-lie guarantee: a missing breakdown never
  prints as a zero.
