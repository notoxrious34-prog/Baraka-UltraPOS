# ADR-078: Information architecture and navigation

Status: ACCEPTED (structure). The visibility matrix and key bindings in docs/design/01-information-architecture.md are provisional until prototype acceptance.
Date: 2026-10-01
Deciders: Architect (Charter v2.0).

## Context
AUD-011 and AUD-011-C1 showed that six fixed-position buttons are the only entry to six functions, that the Utilisateurs and Documents-list screens do not exist, and that there is no command palette. ADR-067 forbids floating buttons.

## Decision
1. The ten constitutional modules are the only top-level destinations: Vente, Documents, Clients & Crédit, Produits & Codes-barres, Stock, Fournisseurs & Achats, Caisse & Clôture, Rapports, Utilisateurs, Paramètres & Sauvegarde.
2. Navigation is a single top bar (ADR-067) plus a navigation rail at the inline-start edge. Labels are shown at 1280 px and wider and icons with tooltips below that width; the user toggle overrides the default.
3. No `position: fixed` element exists except modal overlays, toasts and the command palette. The six floating buttons are removed; their functions move to Documents, Stock and Paramètres & Sauvegarde as mapped in section 5 of the design document.
4. A function lives in the screen that owns its data; contextual actions are row actions or detail-panel actions.
5. Ctrl+K opens a command palette listing every screen and every action allowed to the current role. Ctrl+1..Ctrl+0 select modules and keep their meaning forever.
6. A module has at most 5 tabs.
7. Module visibility derives from the effective permission set once SessionInfo carries it; until then it derives from the role name. Rust remains the authority.

## Consequences
- New screens (Documents list, Customers list/edit, Utilisateurs, Stock overview, Categories) and a toast host and palette must be built; engine work is listed in section 10 of the design document.
- The modal component must provide focus trapping and focus restoration.
- Every later screen specification must name its module, tab, path and role visibility consistently with the design document.
