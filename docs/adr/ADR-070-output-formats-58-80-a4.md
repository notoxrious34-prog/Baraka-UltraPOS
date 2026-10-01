# ADR-070 - Output formats: thermal 58, thermal 80, A4
Status: ACCEPTED (SESSION-ARC-001)
Context: The owner needs thermal printing at both 58 mm and 80 mm, and A4 invoices. The engine stores printer.paper_width_dots=576 as a fixed seed (migration 0006) and renders ESC/POS only. A previous package lists the 58 mm calibration as an open debt.
Decision: Three output profiles: THERMAL_80, THERMAL_58, A4. The thermal width becomes a setting (profile) instead of one fixed constant. A4 documents (Facture, Devis, BL, Avoir and others) carry seller identity (name, address, phone, NIF, NIS, RC, AI), customer block, lines, totals, signature area, and bilingual labels fr/ar. The A4 path does not use ESC/POS; its mechanism (OS print dialog or PDF from the webview) is chosen by a later ADR. The single-source layout principle of ADR-065 is kept for thermal output.
Consequences: Engine work (BRK) for width profiles and A4 data; design work (DSN) for document templates.
Validation gate (OPEN): real thermal printer verification at 58 mm and 80 mm by the owner; legal wording of documents by his accountant.
