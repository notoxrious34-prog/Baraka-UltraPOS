/**
 * 40px cockpit strip: cash session, operator badge, warehouse selector,
 * sync pill, live clock, language and theme toggles. Brand lives in the
 * NavRail only — no duplicate logo/title here (TASK-DSN-048).
 */

import { useEffect, useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useThemeStore } from "@/stores/theme-store";
import { useSessionStore } from "@/stores/session-store";
import { useCartStore } from "@/stores/cart-store";
import { t } from "@/lib/i18n";
import { searchDocuments } from "@/lib/ipc";
import { CashSessionIndicator } from "@/components/pos/CashSessionIndicator";
import { OperationsMenu } from "@/components/pos/OperationsMenu";
import {
  CashIcon,
  ClockIcon,
  DepotIcon,
  LangIcon,
  LogoutIcon,
  MoonIcon,
  SunIcon,
  UserIcon,
} from "@/components/icons";

function useClock(): string {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const timer = window.setInterval(() => {
      setNow(new Date());
    }, 1000);
    return () => {
      window.clearInterval(timer);
    };
  }, []);
  const pad = (value: number): string => value.toString().padStart(2, "0");
  return `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`;
}

export function Topbar(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const toggleLocale = usePosUiStore((state) => state.toggleLocale);
  const openModal = usePosUiStore((state) => state.openModal);
  const activeModal = usePosUiStore((state) => state.activeModal);
  const session = useSessionStore((state) => state.session);
  const warehouses = useSessionStore((state) => state.warehouses);
  const warehouse = useSessionStore((state) => state.warehouse);
  const selectWarehouse = useSessionStore((state) => state.selectWarehouse);
  const logout = useSessionStore((state) => state.logout);
  const clock = useClock();
  const dark = useThemeStore((state) => state.isDark);
  const toggle = useThemeStore((state) => state.toggleTheme);
  const cartLineCount = useCartStore((state) => state.lines.length);
  const [parkedCount, setParkedCount] = useState(0);

  // Parked-draft count: refresh on mount, cart changes (park clears the
  // cart), and modal transitions (resume/discard mutate drafts).
  useEffect(() => {
    const token = useSessionStore.getState().getSessionToken();
    if (token === null) {
      setParkedCount(0);
      return;
    }
    let cancelled = false;
    searchDocuments({
      session_token: token,
      document_type: null,
      status: "DRAFT",
      query: null,
      limit: 100,
    })
      .then((response) => {
        if (!cancelled) {
          setParkedCount(response.documents.length);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setParkedCount(0);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [cartLineCount, activeModal]);

  const depotTitle =
    warehouse !== null ? `${warehouse.code} — ${warehouse.name}` : t(locale, "topbar.noWarehouse");

  return (
    <header className="flex h-10 shrink-0 items-center gap-2 bg-primary-900 px-3 text-white">
      {/* Zone 1 — cash + depot (start-prominent, never hidden). */}
      <div className="flex shrink-0 items-center gap-2">
        <span className="flex items-center gap-1.5">
          <CashIcon className="h-4 w-4 text-white/80" />
          <CashSessionIndicator />
        </span>
        <label className="flex items-center gap-1.5 text-[11px] font-bold text-white/80">
          <DepotIcon className="h-4 w-4 shrink-0" />
          <span className="hidden min-[1101px]:inline">{t(locale, "topbar.warehouse")}</span>
          <select
            value={warehouse !== null ? warehouse.id : ""}
            title={depotTitle}
            onChange={(event) => {
              selectWarehouse(event.target.value);
            }}
            className="max-w-36 rounded-md bg-white/10 px-2 py-1 text-xs font-bold text-white outline-none"
          >
            {warehouse === null && <option value="">{t(locale, "topbar.noWarehouse")}</option>}
            {warehouses
              .filter((candidate) => candidate.isActive)
              .map((candidate) => (
                <option key={candidate.id} value={candidate.id} className="text-slate-900">
                  {candidate.code} — {candidate.name}
                </option>
              ))}
          </select>
        </label>

      </div>

      {/* Zone 2 — center operations + alerts (labels collapse at <=1024px). */}
      <div className="flex min-w-0 flex-1 items-center justify-center gap-2">
        <span className="max-[1024px]:[&_button[aria-haspopup]_span:first-of-type]:hidden">
          <OperationsMenu />
        </span>
        <span className="hidden items-center gap-1 rounded-full bg-emerald-600 px-2 py-1 text-[11px] font-bold sm:flex">
          <span className="inline-block h-2 w-2 rounded-full bg-emerald-200" />
          <span className="max-[1024px]:hidden">{t(locale, "topbar.offline")}</span>
        </span>
        {parkedCount > 0 && (
          <button
            type="button"
            onClick={() => {
              openModal("parked");
            }}
            className="flex shrink-0 items-center gap-1.5 rounded-full bg-amber-500 px-2 py-1 text-[11px] font-bold text-slate-950 hover:bg-amber-400"
          >
            <span aria-hidden="true">F6</span>
            <span className="max-[1024px]:hidden">{t(locale, "parked.badge")}</span>
            <bdi dir="ltr" className="font-mono">
              ({parkedCount})
            </bdi>
          </button>
        )}
      </div>

      {/* Zone 3 — session group (end-quiet; logout always labeled). */}
      <div className="flex shrink-0 items-center gap-2">
        <span className="hidden items-center gap-1.5 rounded-full bg-white/10 px-2 py-1 font-mono text-[11px] font-bold md:flex">
          <UserIcon className="h-4 w-4 text-white/80" />
          {session !== null ? session.username : "—"} · {session !== null ? session.role : "—"}
        </span>

        <span className="hidden items-center gap-1.5 rounded-md bg-white/10 px-2 py-1 font-mono text-[11px] font-bold lg:flex">
          <ClockIcon className="h-4 w-4 text-white/80" />
          <bdi dir="ltr">{clock}</bdi>
        </span>

        <button
          type="button"
          onClick={toggleLocale}
          className="flex items-center gap-1.5 rounded-md bg-white/10 px-2 py-1 text-xs font-bold hover:bg-white/20"
        >
          <LangIcon className="h-4 w-4" />
          {t(locale, "topbar.lang")}
        </button>
        <button
          type="button"
          onClick={toggle}
          aria-label={t(locale, "topbar.theme")}
          className="rounded-md bg-white/10 px-2 py-1 text-xs font-bold hover:bg-white/20"
        >
          {dark ? <SunIcon className="h-4 w-4" /> : <MoonIcon className="h-4 w-4" />}
        </button>
        <button
          type="button"
          onClick={() => {
            void logout();
          }}
          className="flex items-center gap-1.5 rounded-md bg-red-700 px-2 py-1 text-xs font-bold hover:bg-red-600"
        >
          <LogoutIcon className="h-4 w-4" />
          {t(locale, "topbar.logout")}
        </button>
      </div>
    </header>
  );
}
