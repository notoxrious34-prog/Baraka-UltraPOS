# ADR-093: Settings Command Center Two-Tier Navigation & Purified Operational Catalog Layout

- Status: Accepted
- Date: 2026-10-15
- Deciders: Master Principal Architect & Design Lead, Owner

## Context

Administrative and setup tools (Starter Pack 300 items, Category & Rayon
management modal, and CSV Data Exchange) were previously mounted inside
`CatalogModule.tsx`, cluttering the operational selling view.
Simultaneously, `SettingsModule.tsx` used a flat 5-tab bar without clear
grouping, lacking space for catalog tools.

## Decision

1. Transform `SettingsModule.tsx` into a Two-Tier Sidebar Command Center:
   - 4 Logical Groups: Structure & Magasin, Catalogue & Données, Vente &
     Matériel, Sécurité & Système.
   - 8 Dedicated Tabs: Identité, Rayons & Catégories, Pack Quincaillerie,
     Échanges & Données, Matériel & Impression, Fiscalité, Sauvegardes, and
     Diagnostic & À Propos.
2. Extract and embed:
   - `CategorySettingsTab`: Full category CRUD and Algerian standard
     starter rays.
   - `StarterPackSettingsTab`: Depot selection, 1-click batch provisioning,
     and safe dual-track uninstall.
   - `DataExchangeSettingsTab`: CSV catalog import, export, and template
     downloads.
3. Purify `CatalogModule.tsx`:
   - Remove starter pack banner, category modal triggers, and data exchange
     menus.
   - Catalog header focused strictly on instant search, category filter
     pills, and "+ Nouveau Produit".

## Consequences

- Pure separation of daily operational workflows from administrative
  configuration.
- Clean information architecture matching merchant mental models.
- Zero regression in tests; all 581 Vitest tests pass.
