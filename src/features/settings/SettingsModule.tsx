/**
 * Settings command center (DSN-053): two-tier sidebar with 4 logical groups
 * and 8 configuration tabs. Manager-gated in `App.tsx`; ADMIN-only writes
 * stay server-enforced with client-side lock badges.
 */

import { useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { t, type I18nKey } from "@/lib/i18n";
import { StoreIdentityTab } from "./StoreIdentityTab";
import { CategorySettingsTab } from "./CategorySettingsTab";
import { StarterPackSettingsTab } from "./StarterPackSettingsTab";
import { DataExchangeSettingsTab } from "./DataExchangeSettingsTab";
import { PrinterSettingsTab } from "./PrinterSettingsTab";
import { BackupSettingsTab } from "./BackupSettingsTab";
import { FiscalSettingsTab } from "./FiscalSettingsTab";
import { AboutTab } from "./AboutTab";

export type SettingsTab =
  | "identity"
  | "categories"
  | "pack"
  | "exchange"
  | "printer"
  | "fiscal"
  | "backups"
  | "about";

interface TabEntry {
  id: SettingsTab;
  labelKey: I18nKey;
  icon: ReactElement;
  lockedForNonAdmin: boolean;
}

interface TabGroup {
  headerKey: I18nKey;
  tabs: TabEntry[];
}

function sidebarIcon(paths: ReactElement): ReactElement {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.75"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className="shrink-0"
    >
      {paths}
    </svg>
  );
}

const ICON_IDENTITY = sidebarIcon(<path d="M3 9.5 12 4l9 5.5V20H3V9.5Z" />);
const ICON_CATEGORIES = sidebarIcon(<path d="M4 7V5h7l9 9-7 7-9-9Z" />);
const ICON_PACK = sidebarIcon(
  <>
    <path d="M21 8 12 3 3 8v8l9 5 9-5V8Z" />
    <path d="m3 8 9 5 9-5" />
  </>,
);
const ICON_EXCHANGE = sidebarIcon(
  <>
    <path d="M4 8h13l-3-3" />
    <path d="M20 16H7l3 3" />
  </>,
);
const ICON_PRINTER = sidebarIcon(
  <>
    <path d="M7 8V3h10v5" />
    <path d="M4 8h16v8h-3v4H7v-4H4V8Z" />
  </>,
);
const ICON_FISCAL = sidebarIcon(
  <>
    <circle cx="12" cy="12" r="8" />
    <path d="M12 8v8" />
  </>,
);
const ICON_BACKUPS = sidebarIcon(
  <>
    <path d="M12 3 5 6v5c0 5 3.5 8 7 10 3.5-2 7-5 7-10V6l-7-3Z" />
    <path d="m9.5 12 2 2 3.5-4" />
  </>,
);
const ICON_ABOUT = sidebarIcon(
  <>
    <circle cx="12" cy="12" r="8" />
    <path d="M12 11v5" />
    <path d="M12 8h.01" />
  </>,
);

function lockBadge(): ReactElement {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.75"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className="ms-auto shrink-0 opacity-60"
    >
      <rect x="5" y="11" width="14" height="9" rx="2" />
      <path d="M8 11V8a4 4 0 0 1 8 0v3" />
    </svg>
  );
}

const GROUPS: TabGroup[] = [
  {
    headerKey: "settings.groupShop",
    tabs: [
      { id: "identity", labelKey: "settings.tabIdentity", icon: ICON_IDENTITY, lockedForNonAdmin: false },
      { id: "categories", labelKey: "settings.tabCategories", icon: ICON_CATEGORIES, lockedForNonAdmin: false },
    ],
  },
  {
    headerKey: "settings.groupData",
    tabs: [
      { id: "pack", labelKey: "settings.tabPack", icon: ICON_PACK, lockedForNonAdmin: false },
      { id: "exchange", labelKey: "settings.tabExchange", icon: ICON_EXCHANGE, lockedForNonAdmin: false },
    ],
  },
  {
    headerKey: "settings.groupSales",
    tabs: [
      { id: "printer", labelKey: "settings.tabPrinter", icon: ICON_PRINTER, lockedForNonAdmin: false },
      { id: "fiscal", labelKey: "settings.tabFiscal", icon: ICON_FISCAL, lockedForNonAdmin: true },
    ],
  },
  {
    headerKey: "settings.groupSystem",
    tabs: [
      { id: "backups", labelKey: "settings.tabBackups", icon: ICON_BACKUPS, lockedForNonAdmin: true },
      { id: "about", labelKey: "settings.tabAbout", icon: ICON_ABOUT, lockedForNonAdmin: false },
    ],
  },
];

export function SettingsModule(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [tab, setTab] = useState<SettingsTab>("identity");
  const [categoryTick, setCategoryTick] = useState(0);
  const isAdmin = session?.role === "ADMIN";

  const renderTab = (): ReactElement | null => {
    if (session === null) {
      return null;
    }
    const token = session.token;
    switch (tab) {
      case "identity":
        return <StoreIdentityTab locale={locale} />;
      case "categories":
        return (
          <CategorySettingsTab
            sessionToken={token}
            onChanged={() => {
              setCategoryTick((current) => current + 1);
            }}
          />
        );
      case "pack":
        return <StarterPackSettingsTab locale={locale} sessionToken={token} />;
      case "exchange":
        return <DataExchangeSettingsTab locale={locale} sessionToken={token} />;
      case "printer":
        return <PrinterSettingsTab locale={locale} sessionToken={token} />;
      case "fiscal":
        return <FiscalSettingsTab locale={locale} sessionToken={token} isAdmin={isAdmin} />;
      case "backups":
        return <BackupSettingsTab locale={locale} sessionToken={token} isAdmin={isAdmin} />;
      case "about":
        return <AboutTab locale={locale} />;
    }
  };

  return (
    <section className="flex h-full min-h-0 flex-row overflow-hidden bg-slate-50 dark:bg-slate-900">
      <nav
        aria-label={t(locale, "settings.title")}
        className="flex w-56 shrink-0 flex-col gap-1 overflow-y-auto border-e border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-900"
      >
        <h1 className="px-3 pb-1 text-lg font-extrabold">{t(locale, "settings.title")}</h1>
        {GROUPS.map((group) => (
          <div key={group.headerKey}>
            <p className="px-3 pb-1 pt-4 text-[11px] font-bold uppercase tracking-wider text-slate-500">
              {t(locale, group.headerKey)}
            </p>
            <div className="flex flex-col gap-1">
              {group.tabs.map((entry) => {
                const selected = tab === entry.id;
                return (
                  <button
                    key={entry.id}
                    type="button"
                    aria-current={selected ? "page" : undefined}
                    onClick={() => {
                      setTab(entry.id);
                    }}
                    className={`flex items-center gap-2 rounded-lg px-3 py-2 text-start text-xs font-bold ${
                      selected
                        ? "bg-primary-900 text-white"
                        : "text-slate-700 hover:bg-slate-200 dark:text-slate-200 dark:hover:bg-slate-800"
                    }`}
                  >
                    {entry.icon}
                    <span className="min-w-0 flex-1 truncate">
                      {t(locale, entry.labelKey)}
                    </span>
                    {entry.lockedForNonAdmin && !isAdmin ? lockBadge() : null}
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </nav>
      <div
        key={`${tab}-${categoryTick}`}
        className="flex min-h-0 min-w-0 flex-1 flex-col bg-white dark:bg-slate-950"
      >
        {renderTab()}
      </div>
    </section>
  );
}
