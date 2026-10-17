/**
 * Settings & store configuration module (DSN-031): tabbed administrative
 * panel — store identity, printer hardware, backup snapshots (admin-only).
 * Mounted behind the manager gate in `App.tsx`.
 */

import { useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { t } from "@/lib/i18n";
import { StoreIdentityTab } from "./StoreIdentityTab";
import { PrinterSettingsTab } from "./PrinterSettingsTab";
import { BackupSettingsTab } from "./BackupSettingsTab";
import { FiscalSettingsTab } from "./FiscalSettingsTab";
import { AboutTab } from "./AboutTab";

type SettingsTab = "identity" | "printer" | "backups" | "fiscal" | "about";

export function SettingsModule(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [tab, setTab] = useState<SettingsTab>("identity");
  const isAdmin = session?.role === "ADMIN";

  const tabs: Array<{ id: SettingsTab; label: string }> = [
    { id: "identity", label: t(locale, "settings.tabIdentity") },
    { id: "printer", label: t(locale, "settings.tabPrinter") },
    { id: "backups", label: t(locale, "settings.tabBackups") },
    { id: "fiscal", label: t(locale, "settings.tabFiscal") },
    { id: "about", label: t(locale, "settings.tabAbout") },
  ];

  return (
    <section className="flex h-full min-h-0 flex-col overflow-hidden bg-slate-50 dark:bg-slate-900">
      <header className="shrink-0 p-4 pb-2">
        <h1 className="text-lg font-extrabold">{t(locale, "settings.title")}</h1>
        <div className="mt-2 flex gap-2">
          {tabs.map((entry) => (
            <button
              key={entry.id}
              type="button"
              onClick={() => {
                setTab(entry.id);
              }}
              className={`rounded-md px-3 py-1 text-xs font-bold ${
                tab === entry.id
                  ? "bg-accent-500 text-slate-950"
                  : "bg-slate-200 text-slate-700 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-200 dark:hover:bg-slate-600"
              }`}
            >
              {entry.label}
            </button>
          ))}
        </div>
      </header>
      <div className="flex min-h-0 flex-1 flex-col">
        {tab === "identity" && <StoreIdentityTab locale={locale} />}
        {tab === "printer" && session !== null && (
          <PrinterSettingsTab locale={locale} sessionToken={session.token} />
        )}
        {tab === "backups" && session !== null && (
          <BackupSettingsTab locale={locale} sessionToken={session.token} isAdmin={isAdmin} />
        )}
        {tab === "fiscal" && session !== null && (
          <FiscalSettingsTab locale={locale} sessionToken={session.token} isAdmin={isAdmin} />
        )}
        {tab === "about" && <AboutTab locale={locale} />}
      </div>
    </section>
  );
}
