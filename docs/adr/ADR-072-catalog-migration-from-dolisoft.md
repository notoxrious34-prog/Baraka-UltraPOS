# ADR-072 - Catalog migration from Dolisoft export
Status: ACCEPTED (SESSION-ARC-001); file format pending profiling
Context: The owner's product list lives in Dolisoft and can be exported to Excel. The engine has no import command today.
Decision: The initial catalog is imported through an in-app, owner-run import screen backed by an engine import command: file selection, preview, validation report, then one atomic commit with an audit record. Re-running the import is idempotent by product reference or barcode. The product never reads Dolisoft files or databases directly. The exact file format and column mapping are fixed only after a read-only profiling task (AUD) on the real export placed by the owner in D:\Dev\BarakaInbox. Migration of customers, suppliers and opening balances is undecided pending the owner's answer. The INIT-13 demo dataset stays separate and is written only to scratch directories.
Consequences: Import tasks are written after the profiling report, never before.
Validation gate: owner reviews the import preview before commit.
