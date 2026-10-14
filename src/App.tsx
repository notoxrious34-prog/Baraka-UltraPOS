/**
 * Application root. Unauthenticated: LoginScreen. Authenticated: a minimal
 * view-switch rail (TASK-BRK-011; no router dependency — the pre-existing
 * LoginScreen/PosShell toggle extended to a third view) with the sovereign
 * POS Cockpit as default and a manager-only Suppliers module.
 */

import { useEffect, useState, type ReactElement } from "react";
import { useSessionStore } from "@/stores/session-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useAppViewStore, type AppView } from "@/stores/app-view-store";
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

function isManagerRole(role: string): boolean {
  return role === "GERANT" || role === "ADMIN";
}

function NavRail({
  view,
  onSelect,
}: {
  view: AppView;
  onSelect: (view: AppView) => void;
}): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const role = useSessionStore((state) => state.session?.role);
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
      {entry("pos", t(locale, "nav.pos"))}
      {entry("stock", t(locale, "nav.stock"))}
      {entry("customers", t(locale, "nav.customers"))}
      {entry("documents", t(locale, "nav.documents"))}
      {entry("suppliers", t(locale, "nav.suppliers"))}
      {entry("reports", t(locale, "nav.reports"))}
      {entry("catalog", t(locale, "nav.catalog"))}
      {entry("settings", t(locale, "nav.settings"))}
      {role === "ADMIN" && entry("users", t(locale, "nav.users"))}
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

export default function App(): ReactElement {
  const session = useSessionStore((state) => state.session);
  const locale = usePosUiStore((state) => state.locale);
  const view = useAppViewStore((state) => state.view);
  const setView = useAppViewStore((state) => state.setView);
  const manager = session !== null && isManagerRole(session.role);
  const isAdmin = session !== null && session.role === "ADMIN";

  return (
    <div
      dir={locale === "ar-DZ" ? "rtl" : "ltr"}
      lang={locale === "ar-DZ" ? "ar" : "fr"}
      className={`h-full ${locale === "ar-DZ" ? "font-arabic" : "font-sans"}`}
    >
      {session === null ? (
        <UnauthenticatedGate />
      ) : (
        <div className="flex h-full flex-col">
          {manager && <NavRail view={view} onSelect={setView} />}
          <div className="min-h-0 flex-1">
            {view === "stock" && manager ? (
              <StockModule />
            ) : view === "customers" && manager ? (
              <CustomersModule />
            ) : view === "documents" && manager ? (
              <DocumentsModule />
            ) : view === "suppliers" && manager ? (
              <SuppliersModule />
            ) : view === "reports" && manager ? (
              <ReportsHomeScreen />
            ) : view === "catalog" && manager ? (
              <CatalogModule />
            ) : view === "settings" && manager ? (
              <SettingsModule />
            ) : view === "users" ? (
              isAdmin ? (
                <UsersModule />
              ) : (
                <section className="flex h-full items-center justify-center p-8">
                  <p className="rounded-md bg-red-50 px-4 py-3 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
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
