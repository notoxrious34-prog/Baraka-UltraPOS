Title: ADR-075 Catalog import scope: products only
Status: Accepted
Context: The owner decided to start with a fresh customer register instead of migrating customers and their debt balances from Dolisoft. ADR-072 had left the migration of customers, suppliers and opening balances undecided.
Decision: The Dolisoft import covers the product catalog only. No customers, suppliers, debt balances, or stock movement history are migrated. Manual opening balances per customer may be offered later as a separate feature and a separate decision. The handling of opening stock quantities is decided after the profiling of a readable export (AUD-005-C1). This ADR supersedes the "undecided" clause of ADR-072 for customers, suppliers and balances; all other clauses of ADR-072 remain in force.
Consequences: The import mapping and validation report concern products only; the credit ledger starts clean; no customer personal data passes through the import path.
