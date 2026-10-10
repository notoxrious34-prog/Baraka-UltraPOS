/**
 * 56px top bar: brand, operator badge, warehouse selector, sync pill,
 * live clock, language and theme toggles.
 */

import { useEffect, useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { useCartStore } from "@/stores/cart-store";
import { t } from "@/lib/i18n";
import { searchDocuments } from "@/lib/ipc";
import { CashSessionIndicator } from "@/components/pos/CashSessionIndicator";
import { OperationsMenu } from "@/components/pos/OperationsMenu";

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

function useDarkMode(): { dark: boolean; toggle: () => void } {
  const [dark, setDark] = useState(false);
  useEffect(() => {
    document.documentElement.classList.toggle("dark", dark);
  }, [dark]);
  return { dark, toggle: () => setDark((value) => !value) };
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
  const { dark, toggle } = useDarkMode();
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

  return (
    <header className="flex h-14 shrink-0 items-center gap-3 bg-primary-900 px-3 text-white">
      <div className="flex items-center gap-2">
        <span className="flex h-9 w-9 items-center justify-center rounded-lg bg-accent-500 text-lg font-black text-slate-950">
          ب
        </span>
        <div className="leading-tight">
          <p className="text-sm font-extrabold">{t(locale, "app.title")}</p>
          <p className="text-[11px] text-white/70">{t(locale, "app.subtitle")}</p>
        </div>
      </div>

      <div className="ms-auto flex items-center gap-2">
        <CashSessionIndicator />
        <span className="hidden items-center gap-1 rounded-full bg-emerald-600 px-2 py-1 text-[11px] font-bold sm:flex">
          <span className="inline-block h-2 w-2 rounded-full bg-emerald-200" />
          {t(locale, "topbar.offline")}
        </span>

        <label className="flex items-center gap-1 text-[11px] font-bold text-white/80">
          {t(locale, "topbar.warehouse")}
          <select
            value={warehouse !== null ? warehouse.id : ""}
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

        <OperationsMenu />
        {parkedCount > 0 && (
          <button
            type="button"
            onClick={() => {
              openModal("parked");
            }}
            className="flex shrink-0 items-center gap-1 rounded-full bg-amber-500 px-2 py-1 text-[11px] font-bold text-slate-950 hover:bg-amber-400"
          >
            F6 · {t(locale, "parked.badge")} ({parkedCount})
          </button>
        )}

        <span className="hidden rounded-full bg-white/10 px-2 py-1 font-mono text-[11px] font-bold md:block">
          {session !== null ? session.username : "—"} · {session !== null ? session.role : "—"}
        </span>

        <span className="hidden rounded-md bg-white/10 px-2 py-1 font-mono text-[11px] font-bold lg:block">
          {clock}
        </span>

        <button
          type="button"
          onClick={toggleLocale}
          className="rounded-md bg-white/10 px-2 py-1 text-xs font-bold hover:bg-white/20"
        >
          {t(locale, "topbar.lang")}
        </button>
        <button
          type="button"
          onClick={toggle}
          aria-label={t(locale, "topbar.theme")}
          className="rounded-md bg-white/10 px-2 py-1 text-xs font-bold hover:bg-white/20"
        >
          {dark ? "☀" : "☾"}
        </button>
        <button
          type="button"
          onClick={() => {
            void logout();
          }}
          className="rounded-md bg-red-700 px-2 py-1 text-xs font-bold hover:bg-red-600"
        >
          {t(locale, "topbar.logout")}
        </button>
      </div>
    </header>
  );
}
