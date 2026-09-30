/**
 * Cash session indicator for the Topbar (TASK-BRK-010).
 * Replaces the inert placeholder: red CLOSED, green OPEN, amber CLOSING.
 * Refreshes reactively after every mutating command (no manual reload).
 */

import { useEffect, useState, type ReactElement } from "react";
import { useCashSessionStore } from "@/stores/cash-session-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { t } from "@/lib/i18n";
import { formatMoney } from "@/lib/money";
import { ipcCashGetSessionReport } from "@/lib/ipc";

function useLiveTotal(sessionId: string | null, expected: string | null): string | null {
  const session = useSessionStore((state) => state.session);
  const [total, setTotal] = useState<string | null>(expected);
  useEffect(() => {
    if (sessionId === null || session === null) {
      setTotal(expected);
      return;
    }
    let cancelled = false;
    ipcCashGetSessionReport(session.token, sessionId)
      .then((report) => {
        if (cancelled) {
          return;
        }
        let sum = 0n;
        for (const movement of report.movements) {
          sum += BigInt(movement.amountCentimes);
        }
        setTotal(sum.toString());
      })
      .catch(() => {
        if (!cancelled) {
          setTotal(expected);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [sessionId, expected, session]);
  return total;
}

export function CashSessionIndicator(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const openModal = usePosUiStore((state) => state.openModal);
  const session = useSessionStore((state) => state.session);
  const cash = useCashSessionStore((state) => state.session);
  const loading = useCashSessionStore((state) => state.loading);
  const refresh = useCashSessionStore((state) => state.refresh);
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    if (session !== null) {
      void refresh();
    }
  }, [session, refresh]);

  const liveTotal = useLiveTotal(
    cash !== null ? cash.id : null,
    cash !== null ? cash.systemExpectedCentimes : null,
  );

  if (session === null) {
    return <span />;
  }

  if (loading && cash === null) {
    return (
      <span className="rounded-full bg-white/10 px-2 py-1 text-[11px] font-bold">
        {t(locale, "cash.loading")}
      </span>
    );
  }

  if (cash === null || cash.status === "CLOSED") {
    return (
      <span className="flex items-center gap-1 rounded-full bg-red-700 px-2 py-1 text-[11px] font-bold">
        <span className="inline-block h-2 w-2 rounded-full bg-red-200" />
        {t(locale, "cash.closed")}
        <button
          type="button"
          onClick={() => {
            openModal("cash-open");
          }}
          className="rounded-md bg-white/20 px-2 py-0.5 hover:bg-white/30"
        >
          {t(locale, "cash.openSession")}
        </button>
      </span>
    );
  }

  if (cash.status === "CLOSING") {
    return (
      <span className="flex items-center gap-1 rounded-full bg-amber-600 px-2 py-1 text-[11px] font-bold">
        <span className="inline-block h-2 w-2 rounded-full bg-amber-200" />
        {t(locale, "cash.closing")}
        <button
          type="button"
          onClick={() => {
            openModal("cash-close");
          }}
          className="rounded-md bg-white/20 px-2 py-0.5 hover:bg-white/30"
        >
          {t(locale, "cash.closeTitle")}
        </button>
      </span>
    );
  }

  const displayTotal = liveTotal ?? cash.openingFloatCentimes;
  let totalText = "—";
  try {
    totalText = formatMoney(displayTotal, locale);
  } catch {
    totalText = displayTotal;
  }

  return (
    <span className="relative flex items-center gap-1 rounded-full bg-emerald-700 px-2 py-1 text-[11px] font-bold">
      <span className="inline-block h-2 w-2 rounded-full bg-emerald-200" />
      <span className="font-mono tabular-nums">{totalText}</span>
      <button
        type="button"
        aria-label="cash-menu"
        onClick={() => {
          setMenuOpen((value) => !value);
        }}
        className="rounded-md bg-white/20 px-2 py-0.5 hover:bg-white/30"
      >
        ▾
      </button>
      {menuOpen && (
        <span className="absolute end-0 top-full z-50 mt-1 flex min-w-44 flex-col gap-1 rounded-lg bg-white p-2 text-slate-900 shadow-2xl">
          <button
            type="button"
            onClick={() => {
              setMenuOpen(false);
              openModal("cash-expense");
            }}
            className="rounded-md bg-slate-100 px-2 py-1 text-start text-xs font-bold hover:bg-slate-200"
          >
            {t(locale, "cash.recordExpense")}
          </button>
          <button
            type="button"
            onClick={() => {
              setMenuOpen(false);
              openModal("cash-close");
            }}
            className="rounded-md bg-slate-100 px-2 py-1 text-start text-xs font-bold hover:bg-slate-200"
          >
            {t(locale, "cash.closeSession")}
          </button>
        </span>
      )}
    </span>
  );
}
