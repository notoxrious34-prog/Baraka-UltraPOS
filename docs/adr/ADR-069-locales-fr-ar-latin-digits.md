# ADR-069 - Locales fr + ar, Latin digits always
Status: ACCEPTED (SESSION-ARC-001)
Context: The owner works in French and Arabic and accepts Latin digits 0-9 everywhere. The current code defaults to ar-DZ without persistence (src/stores/pos-ui-store.ts:10-36) while the owner's screenshots were in French.
Decision: UI locales are fr-DZ and ar-DZ; English is deferred. Digits are always Latin 0-9 in every locale, on screen and in printed documents. Numbers, currency amounts, codes and dates are LTR-isolated inside RTL text. Arabic UI uses RTL with logical CSS properties. The chosen locale is persisted per device; the first-run default is French; switching is always available from the top bar. Printed documents carry their own language (fr or ar) fixed at creation time.
Consequences: Arabic-Indic digits are never rendered. All UI strings are authored in both languages by the Architect; a glossary lives in docs/design/glossary-ar-fr.md.
Validation gate: none.
