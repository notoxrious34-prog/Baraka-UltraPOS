# TASK-AUD-034 REPORT — Legacy gates in Documents, Settings, role helpers

HEAD `0104e4b`. Standard: ADR-100 (`hasPermission` over live `permissions`;
Rust authoritative). Raw ripgrep: GREP.txt.

## Area 1 — Documents root gate (1 finding)

- D1 — src/features/documents/DocumentsModule.tsx:19-21 (local
  `isManagerRole`), enforced :39
  (`session !== null && isManagerRole(session.role)`); comment header :3-7
  claims "Manager-only". Current behavior: GERANT+ADMIN see the journal;
  Vendeur/Caissier get the red UNAUTHORIZED card (route visible in nav —
  App.tsx:95 `canAccessDocuments = hasPermission("CREATE_SALE")` admits all
  four roles, so the nav promises what the module root denies).
- Engine permission: `documents_search` requires
  `CreateSale OR ReportsView`
  (crates/application/src/use_cases/sales.rs:1312-1313). Both reads
  (`SearchDocumentsUseCase::execute`) and returns context sit behind this
  duality; no Gerant-only document permission exists.
- Recommendation: `session !== null && (hasPermission("CREATE_SALE") ||
  hasPermission("REPORTS_VIEW"))` — the exact App.tsx:95/97 duality.
  Roles delta vs today: Vendeur + Caissier GAIN access (both hold
  CreateSale: role.rs:121,131); Gerant/Admin unchanged; NOBODY loses access.
- UX: **hide route unchanged is wrong; open the view** (permission-gated
  read). No new guidance key needed — the journal is read-only; any future
  mutating document action (reprint/process-return) gates separately
  (ReprintDocument: printing.rs:43; returns use cases carry their own guards).

## Area 2 — Settings admin gates (per-tab matrix)

Root: SettingsModule.tsx:157 `const isAdmin = session?.role === "ADMIN"`;
tabs flagged `lockedForNonAdmin` (:34,125-147: fiscal:true, backups:true);
lock badge :224; `isAdmin` prop drilled to FiscalSettingsTab (:183) and
BackupSettingsTab (:185). Stale pin settings-module.test.ts:76 asserts
`session?.role === "ADMIN"` — must be re-pinned to the permission expression.

| Tab | Read | Write (engine citation) | Non-ADMIN UX |
|---|---|---|---|
| identity | any authenticated session (get) | `SystemSettingsManage`, Admin only (ipc.rs:2922, SaveStoreIdentityUseCase) | route visible; save disabled + guidance (MISSING-KEY — no settings.*ReservedGuidance exists yet) |
| categories | PosSell reads (catalog.rs:417…) | `StockAdjustment OR SystemSettingsManage` (MANAGE_PERMISSIONS, catalog.rs:19-21; enforced :568/:612) | route visible; create/edit disabled + guidance (MISSING-KEY) |
| pack (starter seed) | — | same MANAGE_PERMISSIONS (~catalog.rs:707) | visible; seed disabled + guidance (MISSING-KEY) |
| exchange (import) | — | same MANAGE_PERMISSIONS (~catalog.rs:918) | visible; import disabled + guidance (MISSING-KEY) |
| printer | `SystemSettingsManage` (printing.rs:187 GetPrintStatus / :337) | transport config `ManagePrinterSettings`, Admin only (printing.rs:442, UpdatePrinterSettingsUseCase :416) | visible; test-print/update disabled + guidance (MISSING-KEY). NOTE Gerant lacks ManagePrinterSettings (role.rs:150-161) — Gerant also loses printer writes vs any role-name assumption. |
| fiscal | form renders for all (FiscalSettingsTab) | `SystemSettingsManage`, Admin only (sales.rs:1395) | keep route visible; toggle/save already `disabled={saving \|\| !isAdmin}` (:53,86) → rewire to `!hasPermission("SYSTEM_SETTINGS_MANAGE")` + guidance (MISSING-KEY) |
| backups | hard hide when !isAdmin (BackupSettingsTab.tsx:61-69, note `settings.backupRestricted`) | `SystemSettingsManage`, Admin only (backup.rs:261) | route KEEP-VISIBLE-BUT-LOCKED is today's pattern; Options: (a) keep hide + badge (only hide in the app, justified: destructive restore), or (b) Disabled-with-Guidance for consistency. Existing key `settings.backupRestricted` FR (i18n.ts:173) + AR (i18n.ts:1229) already covers the note. |
| about | none | none | visible all |

- `lockedForNonAdmin` badge: KEEP the badge mechanism, rewire its predicate
  from `!isAdmin` to the per-tab write permission (fiscal/backups →
  `!hasPermission("SYSTEM_SETTINGS_MANAGE")`).
- `isAdmin` prop (both tabs): REMOVE the boolean prop; pass nothing —
  each tab reads `hasPermission` from the session store directly (kills the
  role-string drilling; matches DSN-077 container pattern).

## Area 3 — Role helpers (warehouse-transfer-store.ts:63-68)

- H1 `isAdminRole` (:63-65), H2 `isManagerRole` (:67-69): pure role-string
  predicates, superseded by ADR-100.
- Call sites (exhaustive, GREP.txt): WarehouseTransferModal.tsx:17-18
  (imports), :292-293 (`manager`/`admin` consts) — ONLY production consumer.
  Test-only importers: warehouse-transfer-modal.test.ts,
  safe-delete-modal.test.ts. OperationsMenu.tsx and StockModule.tsx import
  the STORE hook, not the helpers. No other src consumer exists.
- Verdict: H1/H2 REMOVE after WarehouseTransferModal :292-293 is rewired to
  `hasPermission("TRANSFER_STOCK")` (engine: stock_transfer requires
  TransferStock; create-warehouse inside the modal requires
  ManageWarehouses). Modal-internal manager/admin branches become
  `canTransfer` / `canManageWarehouses` disabled-with-guidance — same UX
  pattern as StockModule (DSN-077). Update the two test files' pins.

## Edge cases

- Null session: all recommended expressions short-circuit on
  `session !== null` first (DSN-077 pattern); store's `hasPermission`
  reads live `get().permissions` (session-store.ts:104) — no stale closure.
- Direct-URL access: routes render through App.tsx view switch
  (App.tsx:250-251 documents pattern); recommended root gates deny with the
  standard UNAUTHORIZED card, Rust use cases deny independently.
- RTL/FR-AR: guidance titles flow through `t(locale, …)` + existing
  `dir="ltr"`/bidi conventions; 5 new guidance keys are MISSING
  (only `settings.backupRestricted` exists FR+AR) — DSN-phase must add
  settings.*ReservedGuidance FR+AR before wiring `title=` to them.
- Stale pin: settings-module.test.ts:76 (`session?.role === "ADMIN"`) —
  re-pin to `hasPermission("SYSTEM_SETTINGS_MANAGE")` + `lockedForNonAdmin`
  predicate pins; same treatment as DSN-077-C1's stock-module.test.ts fix.

## Engine citations (every permission claim)

- CreateSale‖ReportsView for documents_search: sales.rs:1312-1313.
- PosSell catalog reads: catalog.rs:417,462,489,673.
- MANAGE_PERMISSIONS [StockAdjustment, SystemSettingsManage]: catalog.rs:19-21;
  require_any: catalog.rs:29; enforced ≈:568/:612/:707/:918.
- SystemSettingsManage → backups: backup.rs:261; fiscal write: sales.rs:1395;
  identity save: ipc.rs:2922; print status/settings reads: printing.rs:187,337.
- ManagePrinterSettings → printer transport: printing.rs:442.
- ReprintDocument: printing.rs:43.
- Role holders: Vendeur matrix role.rs:121; Caissier :131-135; Gerant
  :150-161 (no SystemSettingsManage/ManageWarehouses); Admin :169-197 (all).
  Enum values role.rs:28-65 (34 total).
