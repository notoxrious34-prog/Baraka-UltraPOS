/**
 * Application root. Unauthenticated: LoginScreen. Authenticated: a minimal
 * view-switch rail (TASK-BRK-011; no router dependency — the pre-existing
 * LoginScreen/PosShell toggle extended to a third view) with the sovereign
 * POS Cockpit as default.
 *
 * Navigation gating (TASK-DSN-072, Batch 1): every rail entry and route is
 * decided exclusively by `hasPermission(...)` through the pure
 * `canAccessView` helper below — zero role-string comparisons. Engine remains
 * authoritative per action; the shell only hides unreachable views.
 */

import { useEffect, useState, type ReactElement } from "react";
import { useSessionStore } from "@/stores/session-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useAppViewStore, type AppView } from "@/stores/app-view-store";
import { useThemeStore } from "@/stores/theme-store";
import { PosShell } from "@/components/pos/PosShell";
import { LoginScreen } from "@/features/auth/LoginScreen";
import { FirstRunSetupScreen } from "@/features/auth/FirstRunSetupScreen";
import { SuppliersModule } from "@/features/suppliers/SupplierListScreen";
import { CustomersModule } from "@/features/customers/CustomersModule";
import { DocumentsModule } from "@/features/documents/DocumentsModule";
import { StockModule } from "@/features/stock/StockModule";
import { UsersModule } from "@/features/users/UsersModule";
import { SettingsModule } from "@/features/settings/SettingsModule";
import { ReportsHomeScreen } from "@/features/reports/ReportsHomeScreen";
import { CatalogModule } from "@/features/catalog/CatalogModule";
import { ipcCheckFirstRunStatus } from "@/lib/ipc";
import { t } from "@/lib/i18n";
import barakaLogoIcon from "@/assets/logo/baraka-logo-icon.png";

/**
 * Pure view-access decision (TASK-DSN-072): maps each top-level view to its
 * domain capability. Deviations from a single-permission gate are engine-cited:
 * - documents: engine search = `CreateSale` OR `ReportsView` (sales.rs:1313);
 *   `CreateSale` alone covers all four roles, so one check is exact.
 * - reports: `REPORTS_VIEW` (all 10 reports.rs use cases) OR `CASH_REPORT_VIEW`
 *   (single-session report, cash.rs:378) — admits Caissier to the route while
 *   the engine still guards each report individually.
 * - settings: `SYSTEM_SETTINGS_MANAGE` (identity/printer/fiscal writes) OR
 *   `STOCK_ADJUSTMENT` (categories tab via catalog MANAGE_PERMISSIONS) —
 *   keeps Gerant on its legitimate categories tab; engine guards each tab.
 */
export function canAccessView(view: AppView, permissions: readonly string[]): boolean {
  const has = (permission: string): boolean => permissions.includes(permission);
  switch (view) {
    case "pos":
      return has("CREATE_SALE");
    case "catalog":
      return has("VIEW_STOCK");
    case "stock":
      return has("VIEW_STOCK");
    case "customers":
      // Vendeur holds VIEW_CUSTOMERS (role.rs:121) and engine reads are open
      // (customers.rs:250,283,313,396) — the grant is intentional, not a leak.
      return has("VIEW_CUSTOMERS");
    case "documents":
      return has("CREATE_SALE");
    case "suppliers":
      return has("SUPPLIER_MANAGE");
    case "reports":
      return has("REPORTS_VIEW") || has("CASH_REPORT_VIEW");
    case "settings":
      return has("SYSTEM_SETTINGS_MANAGE") || has("STOCK_ADJUSTMENT");
    case "users":
      return has("USER_MANAGE");
  }
}

/** Standardized unauthorized fallback (semantic tokens, both themes). */
function AccessDenied({ locale }: { locale: "fr-DZ" | "ar-DZ" }): ReactElement {
  return (
    <div className="flex h-full items-center justify-center bg-surface-canvas p-6">
      <div className="max-w-md rounded-lg border border-danger-fg/30 bg-danger-bg p-6 text-center text-danger-fg">
        <p className="font-semibold">{t(locale, "common.accessDenied")}</p>
      </div>
    </div>
  );
}

export function NavRail({
  view,
  onSelect,
}: {
  view: AppView;
  onSelect: (view: AppView) => void;
}): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const hasPermission = useSessionStore((state) => state.hasPermission);
  const canAccessPos = hasPermission("CREATE_SALE");
  const canAccessCatalog = hasPermission("VIEW_STOCK");
  const canAccessStock = hasPermission("VIEW_STOCK");
  const canAccessCustomers = hasPermission("VIEW_CUSTOMERS");
  const canAccessDocuments = hasPermission("CREATE_SALE");
  const canAccessSuppliers = hasPermission("SUPPLIER_MANAGE");
  const canAccessReports = hasPermission("REPORTS_VIEW") || hasPermission("CASH_REPORT_VIEW");
  const canAccessSettings =
    hasPermission("SYSTEM_SETTINGS_MANAGE") || hasPermission("STOCK_ADJUSTMENT");
  const canAccessUsers = hasPermission("USER_MANAGE");
  const entry = (target: AppView, label: string): ReactElement => (
    <button
      type="button"
      onClick={() => {
        onSelect(target);
      }}
      className={`rounded-md px-3 py-1 text-xs font-bold ${
        view === target ? "bg-accent-500 text-slate-950" : "bg-white/10 text-white hover:bg-white/20"
      }`}
    >
      {label}
    </button>
  );
  return (
    <nav className="flex h-10 shrink-0 items-center gap-2 bg-primary-900 px-3">
      <span className="flex items-center gap-1.5 pe-1">
        <img
          src={barakaLogoIcon}
          alt="Baraka"
          className="h-7 w-7 select-none object-contain drop-shadow-sm"
        />
        <span className="text-xs font-black">
          <span className="text-amber-300">Baraka</span>
        </span>
      </span>
      {canAccessPos && entry("pos", t(locale, "nav.pos"))}
      {canAccessCatalog && entry("catalog", t(locale, "nav.catalog"))}
      {canAccessStock && entry("stock", t(locale, "nav.stock"))}
      {canAccessCustomers && entry("customers", t(locale, "nav.customers"))}
      {canAccessDocuments && entry("documents", t(locale, "nav.documents"))}
      {canAccessSuppliers && entry("suppliers", t(locale, "nav.suppliers"))}
      {canAccessReports && entry("reports", t(locale, "nav.reports"))}
      {canAccessSettings && entry("settings", t(locale, "nav.settings"))}
      {canAccessUsers && entry("users", t(locale, "nav.users"))}
    </nav>
  );
}

/**
 * Unauthenticated entry point: probes the backend once for first-run
 * status, then shows the setup screen (empty users table) or the normal
 * login form. While the probe is in flight, a minimal loading state reuses
 * the existing `cash.loading` string and the login screen layout.
 */
function UnauthenticatedGate(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const [requiresSetup, setRequiresSetup] = useState<boolean | null>(null);
  useEffect(() => {
    let cancelled = false;
    ipcCheckFirstRunStatus()
      .then((value) => {
        if (!cancelled) {
          setRequiresSetup(value);
        }
      })
      .catch(() => {
        // Backend unreachable (e.g. browser preview without Tauri):
        // preserve the pre-existing behavior and show the login form.
        if (!cancelled) {
          setRequiresSetup(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);
  if (requiresSetup === null) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-4 bg-primary-900 p-6">
        <p className="text-sm font-bold text-white/70">{t(locale, "cash.loading")}</p>
      </div>
    );
  }
  return requiresSetup ? <FirstRunSetupScreen /> : <LoginScreen />;
}

/**
 * Testable shell (TASK-DSN-072): store wiring is limited to live-read stable
 * references (`getSessionToken`, `hasPermission` — both close over `get()`,
 * so they stay observable under SSR/static markup where value selectors
 * freeze on initial state). The active view arrives as a prop, so route
 * protection is directly assertable per role x view.
 */
export function AppShell({
  view,
  onSelectView,
}: {
  view: AppView;
  onSelectView: (view: AppView) => void;
}): ReactElement {
  // Live-read accessor (stable reference): unlike the `session` value, this
  // stays observable under SSR/static markup because the closure reads the
  // current store via `get()`. Null token ⟺ no session (token always set on
  // login/bootstrap; see session-store `getSessionToken`).
  const getSessionToken = useSessionStore((state) => state.getSessionToken);
  // Reactive pin (AUD-031): `getSessionToken`/`hasPermission` are stable
  // function references, so selecting them alone NEVER re-renders when the
  // session object identity changes. Subscribing to `session` forces an
  // immediate re-render on login/logout; the token check below covers
  // SSR/static markup, where value selectors freeze on initial state.
  useSessionStore((state) => state.session);
  const locale = usePosUiStore((state) => state.locale);
  // NOTE: value selectors read zustand initial state under SSR/static markup;
  // `hasPermission` reads the live store via `get()`. Route flags therefore
  // use `hasPermission(...)` verbatim (spec sketch), mirroring `canAccessView`.
  const hasPermission = useSessionStore((state) => state.hasPermission);
  const canAccessPos = hasPermission("CREATE_SALE");
  const canAccessCatalog = hasPermission("VIEW_STOCK");
  const canAccessStock = hasPermission("VIEW_STOCK");
  const canAccessCustomers = hasPermission("VIEW_CUSTOMERS");
  const canAccessDocuments = hasPermission("CREATE_SALE");
  const canAccessSuppliers = hasPermission("SUPPLIER_MANAGE");
  const canAccessReports = hasPermission("REPORTS_VIEW") || hasPermission("CASH_REPORT_VIEW");
  const canAccessSettings =
    hasPermission("SYSTEM_SETTINGS_MANAGE") || hasPermission("STOCK_ADJUSTMENT");
  // Kept name: DSN-071 regression pins assert `canManageUsers` in this file.
  const canManageUsers = hasPermission("USER_MANAGE");

  return (
    <div
      dir={locale === "ar-DZ" ? "rtl" : "ltr"}
      lang={locale === "ar-DZ" ? "ar" : "fr"}
      className={`h-full ${locale === "ar-DZ" ? "font-arabic" : "font-sans"}`}
    >
      {getSessionToken() === null ? (
        <UnauthenticatedGate />
      ) : (
        <div className="flex h-full flex-col">
          <NavRail view={view} onSelect={onSelectView} />
          <div className="min-h-0 flex-1">
            {view === "pos" ? (
              canAccessPos ? (
                <PosShell />
              ) : (
                <AccessDenied locale={locale} />
              )
            ) : view === "stock" ? (
              canAccessStock ? (
                <StockModule />
              ) : (
                <AccessDenied locale={locale} />
              )
            ) : view === "customers" ? (
              canAccessCustomers ? (
                <CustomersModule />
              ) : (
                <AccessDenied locale={locale} />
              )
            ) : view === "documents" ? (
              canAccessDocuments ? (
                <DocumentsModule />
              ) : (
                <AccessDenied locale={locale} />
              )
            ) : view === "suppliers" ? (
              canAccessSuppliers ? (
                <SuppliersModule />
              ) : (
                <AccessDenied locale={locale} />
              )
            ) : view === "reports" ? (
              canAccessReports ? (
                <ReportsHomeScreen />
              ) : (
                <AccessDenied locale={locale} />
              )
            ) : view === "catalog" ? (
              canAccessCatalog ? (
                <CatalogModule />
              ) : (
                <AccessDenied locale={locale} />
              )
            ) : view === "settings" ? (
              canAccessSettings ? (
                <SettingsModule />
              ) : (
                <AccessDenied locale={locale} />
              )
            ) : view === "users" ? (
              canManageUsers ? (
                <UsersModule />
              ) : (
                <section className="flex h-full items-center justify-center bg-surface-canvas p-8">
                  <p className="rounded-md border border-danger-fg/30 bg-danger-bg px-4 py-3 text-sm font-bold text-danger-fg">
                    {t(locale, "users.unauthorized")}
                  </p>
                </section>
              )
            ) : (
              <PosShell />
            )}
          </div>
        </div>
      )}
    </div>
  );
}

export default function App(): ReactElement {
  const view = useAppViewStore((state) => state.view);
  const setView = useAppViewStore((state) => state.setView);

  // Restore the persisted theme preference on startup (TASK-DSN-065).
  useEffect(() => {
    useThemeStore.getState().initTheme();
  }, []);

  return <AppShell view={view} onSelectView={setView} />;
}
