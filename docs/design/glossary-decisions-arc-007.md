# Glossary Terminology Decisions — SESSION-ARC-007

Status: DECIDED (standalone record; NOT YET MERGED into docs/design/glossary-ar-fr.md)
Date: 2026-10-06
Session: SESSION-ARC-007, Reply 19
Owner directive applied (verbatim intent, SESSION-ARC-007, between Reply 14
and Reply 15): terms must match those recognized in comparable international
software (POS/ERP Arabic localizations), or, failing that, use professional
academic technical translation from French to Arabic — not colloquial/dialect
borrowings.

This document resolves the five terms flagged as open in
CTP-SESSION-ARC-007.md Section 12 item 5 (originally presented to the owner
in SESSION-ARC-006 Reply 9). It is a standalone decision record. Merging
these five entries into the master glossary file
(docs/design/glossary-ar-fr.md, ~112 terms, status "v0.1 PROPOSED" as of the
last known audit) is explicitly deferred to a future task, because that
file's exact current format/structure has not been audited in this session
and must not be guessed.

## 1. Clients & Crédit (module name)

- French: Clients & Crédit
- REJECTED: الزبائن والديون / الزبائن والكريدي ("الكريدي" is a French
  loanword in Algerian dialect, not formal/academic Arabic; "الديون" reads
  as "debts," a negative framing not used by comparable ERP/POS products)
- DECIDED: العملاء والائتمان (Al-'Umala' wal-I'timan)
- Rationale: "الائتمان" is the standard accounting/financial Arabic term for
  "crédit" (cf. "حد الائتمان" = credit limit, "بيع بالائتمان" = sale on
  credit), used in recognized Arabic-localized business software (Odoo
  Arabic, SAP Arabic). "العملاء" is the standard formal term for "clients"
  in the same class of software, more formal/consistent than the
  colloquial-leaning "الزبائن".

## 2. Administrateur vs Gérant (role names)

- French: Administrateur / Gérant
- REJECTED (as a pair): المسؤول / المسيّر — flagged in SESSION-ARC-006 as
  carrying a confusion risk between two close-sounding/close-meaning words
  in daily shop use
- DECIDED: مسؤول النظام (Administrateur) / المدير (Gérant)
- Rationale: "مسؤول النظام" ("System Administrator") is the term used in
  recognized Arabic-localized software (operating systems, ERPs) for the
  highest-privilege technical role, and the added qualifier "النظام"
  removes any ambiguity with "المدير" (the store/business manager role,
  "Gérant"), which is the standard Arabic term for an operational manager
  in Arabic-localized business software.

## 3. Avoir (document type)

- French: Avoir
- REJECTED: وصل إرجاع ("return receipt" — not a recognized formal
  accounting term, reads as a generic slip rather than a financial
  instrument)
- DECIDED: إشعار دائن (Ish'ar Da'in)
- Rationale: "إشعار دائن" is the globally recognized formal accounting term
  for "Credit Note" / "Avoir," used consistently across Arabic-localized
  accounting and ERP software (Odoo Arabic, SAP Arabic, and Arabic
  accounting standards terminology).

## 4. À crédit (payment method label)

- French: À crédit
- REJECTED: كريدي (direct French loanword, Algerian dialect usage, not
  formal/academic Arabic)
- DECIDED: بالأجل (Bil-Ajal)
- Rationale: "بالأجل" ("deferred" / "on credit") is the classical,
  well-established Arabic commercial and financial term (cf. "بيع بالأجل" =
  sale on credit/deferred payment), used in formal commercial, accounting,
  and Islamic-finance contexts across the Arabic-speaking world, and is the
  term used in Arabic-localized POS/ERP payment-method lists for this
  payment type.

## 5. Preuve d'encaissement (debt payment receipt)

- French: Preuve d'encaissement / Reçu de paiement
- REJECTED: وصل تسديد / وصل دفع ("وصل" is a generic, informal word for
  "slip/stub," not a term used in recognized accounting software)
- DECIDED: سند قبض (Sanad Qabd)
- Rationale: "سند قبض" ("Receipt Voucher") is the precise, globally
  recognized accounting term for a document evidencing money collected,
  used consistently across major Arabic-localized accounting/ERP software
  (Odoo Arabic: "سند قبض" for Payment Receipt Voucher; SAP Arabic uses the
  same term). It correctly frames the document as a formal accounting
  voucher rather than a generic slip.

## Status and Next Steps

These five decisions are ACCEPTED by the architect under the owner's
explicit terminology criterion (SESSION-ARC-007, between Reply 14 and Reply
15) and are considered final unless the owner raises a specific objection
after reviewing this document. They are NOT YET reflected in:
- docs/design/glossary-ar-fr.md (the ~112-term master glossary file;
  merging these five entries requires a prior literal audit of that file's
  exact table/section format, deferred to a future task)
- Any of the 10 accepted static prototypes (docs/design/prototypes/*.html),
  which predate this terminology decision and may use earlier wording for
  these five concepts; reconciling prototype wording against this decision
  (if any prototype uses conflicting wording) is also deferred to a future
  task and should be checked explicitly before any future prototype
  revision cycle.
- Any shipped UI string, since no production UI screens using these terms
  have been built yet in this project's redesign (per CHARTER-v2.0's
  roadmap, screen implementation has not yet started).

## References

- CHARTER-v2.0, Section 7.8 (Arabic quality, glossary file requirement),
  Section 2.2 (language rules).
- CTP-SESSION-ARC-007.md, Section 12 item 5.
- Owner directive, SESSION-ARC-007, between Reply 14 and Reply 15.
