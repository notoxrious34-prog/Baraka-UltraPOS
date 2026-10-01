# ADR-067 - Single top bar; no floating action buttons
Status: ACCEPTED (SESSION-ARC-001)
Context: AUD-003: the POS shell has two stacked header rows and six fixed-position buttons (Identite & Impression, Sauvegardes & Securite, Regularisation de Stock, Gestion des Depots & Transferts, Retour & Facture d'Avoir, Convertir) that cover the cart and totals (src/components/pos/PosShell.tsx:74-160).
Decision: One top bar of about 48 px replaces the two header rows. Floating action buttons are removed. No engine functionality is removed: identity and printing plus backup and restore move to a Settings screen; stock adjustment and transfers move to the Stock screen; returns, credit notes and conversions move to the Documents screen. Dialogs may remain as overlay modals opened from those screens. position: fixed is allowed only for modal overlays, toasts and the command palette.
Consequences: DSN tasks rebuild the shell and screens on the existing IPC commands. Engine commands stay unchanged unless a BRK task says otherwise.
Validation gate: owner visual acceptance of static prototypes before React work.
