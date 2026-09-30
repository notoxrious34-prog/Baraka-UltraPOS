/**
 * Close cash session modal: two-phase closing (begin -> confirm).
 * Step 1 calls cash.beginClosing on open and shows the frozen expected
 * total read-only. Step 2 collects the counted amount + optional notes,
 * previews variance client-side (display only), and seals via
 * cash.confirmClosing. Cancel calls cash.cancelClosing.
 */

import { useEffect, useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { useCashSessionStore } from "@/stores/cash-session-store";
import { t, translateBackendError } from "@/lib/i18n";
import { formatMoney, newCommandId, parseMoneyToCentimes } from "@/lib/money";
import {
  ipcCashBeginClosing,
  ipcCashCancelClosing,
  ipcCashConfirmClosing,
} from "@/lib/ipc";
import { ModalShell } from "@/components/pos/modals/ModalShell";

function formatOrRaw(centimes: string | null, locale: "ar-DZ" | "fr-DZ"): string {
  if (centimes === null) {
    return "—";
  }
  try {
    return formatMoney(centimes, locale);
  } catch {
    return centimes;
  }
}

function isValidAmount(text: string): boolean {
  if (text.trim() === "") {
    return false;
  }
  try {
    return BigInt(parseMoneyToCentimes(text)) >= 0n;
  } catch {
    return false;
  }
}

export function CloseCashSessionModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const session = useSessionStore((state) => state.session);
  const cash = useCashSessionStore((state) => state.session);
  const setSession = useCashSessionStore((state) => state.setSession);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [countedText, setCountedText] = useState("");
  const [notes, setNotes] = useState("");
  const [closed, setClosed] = useState(false);

  // Step 1: when opened on an OPEN session, begin closing immediately.
  useEffect(() => {
    if (session === null || cash === null || cash.status !== "OPEN") {
      return;
    }
    setBusy(true);
    setError(null);
    ipcCashBeginClosing(session.token, newCommandId())
      .then((updated) => {
        setSession(updated);
      })
      .catch((failure: unknown) => {
        const code = typeof failure === "string" ? failure : "INTERNAL_ERROR";
        setError(translateBackendError(code, locale));
      })
      .finally(() => {
        setBusy(false);
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const cancelClosing = (): void => {
    if (session === null) {
      return;
    }
    setBusy(true);
    setError(null);
    ipcCashCancelClosing(session.token, newCommandId())
      .then((updated) => {
        setSession(updated);
        closeModal();
      })
      .catch((failure: unknown) => {
        const code = typeof failure === "string" ? failure : "INTERNAL_ERROR";
        setError(translateBackendError(code, locale));
      })
      .finally(() => {
        setBusy(false);
      });
  };

  const confirmClosing = (): void => {
    if (session === null || cash === null || !isValidAmount(countedText)) {
      return;
    }
    let centimes = "0";
    try {
      centimes = parseMoneyToCentimes(countedText);
    } catch {
      return;
    }
    setBusy(true);
    setError(null);
    ipcCashConfirmClosing(
      session.token,
      cash.id,
      centimes,
      notes.trim() === "" ? null : notes.trim(),
      newCommandId(),
    )
      .then((updated) => {
        setSession(updated);
        setClosed(true);
      })
      .catch((failure: unknown) => {
        const code = typeof failure === "string" ? failure : "INTERNAL_ERROR";
        setError(translateBackendError(code, locale));
      })
      .finally(() => {
        setBusy(false);
      });
  };

  if (cash === null) {
    return (
      <ModalShell title={t(locale, "cash.closeTitle")} onClose={closeModal}>
        <p className="text-sm font-bold">{t(locale, "cash.loading")}</p>
      </ModalShell>
    );
  }

  if (closed) {
    return (
      <ModalShell title={t(locale, "cash.closeTitle")} onClose={closeModal}>
        <div className="flex flex-col gap-2 text-sm">
          <p className="rounded-lg bg-emerald-50 px-3 py-3 text-center text-base font-black text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
            {t(locale, "cash.closedSummary")}
          </p>
          <div className="flex justify-between">
            <span>{t(locale, "cash.expected")}</span>
            <span className="font-mono font-bold">
              {formatOrRaw(cash.systemExpectedCentimes, locale)}
            </span>
          </div>
          <div className="flex justify-between">
            <span>{t(locale, "cash.counted")}</span>
            <span className="font-mono font-bold">
              {formatOrRaw(cash.actualCountedCentimes, locale)}
            </span>
          </div>
          <div className="flex justify-between">
            <span>{t(locale, "cash.variance")}</span>
            <span className="font-mono font-bold">
              {formatOrRaw(cash.varianceCentimes, locale)}
            </span>
          </div>
          <button
            type="button"
            onClick={closeModal}
            className="mt-2 h-12 rounded-lg bg-accent-500 text-base font-black text-slate-950 hover:bg-amber-400"
          >
            {t(locale, "cash.confirm")}
          </button>
        </div>
      </ModalShell>
    );
  }

  let varianceText = "—";
  let varianceClass = "text-slate-500";
  try {
    const expected = BigInt(cash.systemExpectedCentimes ?? "0");
    if (isValidAmount(countedText)) {
      const counted = BigInt(parseMoneyToCentimes(countedText));
      const variance = counted - expected;
      varianceText = formatMoney(variance.toString(), locale);
      if (variance === 0n) {
        varianceClass = "text-emerald-700 dark:text-emerald-300";
      } else if (variance < 0n ? -variance <= 1_000n : variance <= 1_000n) {
        varianceClass = "text-amber-700 dark:text-amber-300";
      } else {
        varianceClass = "text-red-700 dark:text-red-300";
      }
    }
  } catch {
    varianceText = "—";
  }

  return (
    <ModalShell title={t(locale, "cash.closeTitle")} onClose={closeModal}>
      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-between rounded-lg bg-slate-100 px-3 py-2 text-sm dark:bg-slate-800">
          <span className="font-bold">{t(locale, "cash.expected")}</span>
          <span className="font-mono text-base font-black tabular-nums">
            {formatOrRaw(cash.systemExpectedCentimes, locale)}
          </span>
        </div>
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold text-slate-500">{t(locale, "cash.counted")}</span>
          <input
            value={countedText}
            onChange={(event) => {
              setCountedText(event.target.value);
            }}
            inputMode="decimal"
            autoComplete="off"
            className="h-11 rounded-lg border border-slate-300 bg-white px-3 font-mono text-base font-black outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800"
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold text-slate-500">{t(locale, "cash.notes")}</span>
          <textarea
            value={notes}
            onChange={(event) => {
              setNotes(event.target.value);
            }}
            rows={2}
            className="rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm font-bold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800"
          />
        </label>
        <div className="flex items-center justify-between rounded-lg bg-slate-50 px-3 py-2 text-sm dark:bg-slate-800/60">
          <span className="font-bold">{t(locale, "cash.variance")}</span>
          <span className={`font-mono text-base font-black tabular-nums ${varianceClass}`}>
            {varianceText}
          </span>
        </div>
        {error !== null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {error}
          </p>
        )}
        <div className="flex gap-2">
          <button
            type="button"
            onClick={cancelClosing}
            disabled={busy}
            className="h-12 flex-1 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 disabled:opacity-40 dark:bg-slate-700"
          >
            {t(locale, "cash.cancelClosing")}
          </button>
          <button
            type="button"
            onClick={() => {
              if (cash.status === "OPEN") {
                closeModal();
              } else {
                confirmClosing();
              }
            }}
            disabled={busy || cash.status !== "CLOSING" || !isValidAmount(countedText)}
            className="h-12 flex-[2] rounded-lg bg-accent-500 text-base font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
          >
            {cash.status === "OPEN"
              ? t(locale, "cash.continueCounting")
              : t(locale, "cash.confirm")}
          </button>
        </div>
      </div>
    </ModalShell>
  );
}
