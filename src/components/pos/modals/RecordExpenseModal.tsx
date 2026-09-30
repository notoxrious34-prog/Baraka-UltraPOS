/**
 * Record drawer expense modal: positive amount + required description.
 * Reachable only via the CashSessionIndicator menu (no shortcut).
 */

import { useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { useCashSessionStore } from "@/stores/cash-session-store";
import { t, translateBackendError } from "@/lib/i18n";
import { formatMoney, newCommandId, parseMoneyToCentimes } from "@/lib/money";
import { ipcCashGetSessionReport, ipcCashRecordExpense } from "@/lib/ipc";
import { ModalShell } from "@/components/pos/modals/ModalShell";

function isValidAmount(text: string): boolean {
  if (text.trim() === "") {
    return false;
  }
  try {
    return BigInt(parseMoneyToCentimes(text)) > 0n;
  } catch {
    return false;
  }
}

export function RecordExpenseModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const session = useSessionStore((state) => state.session);
  const refresh = useCashSessionStore((state) => state.refresh);
  const [amountText, setAmountText] = useState("");
  const [description, setDescription] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);

  const trimmedDescription = description.trim();
  const valid =
    isValidAmount(amountText) &&
    trimmedDescription !== "" &&
    trimmedDescription.length <= 255;
  const canConfirm = !busy && valid && session !== null;

  const confirm = (): void => {
    if (!canConfirm || session === null) {
      return;
    }
    let centimes = "0";
    try {
      centimes = parseMoneyToCentimes(amountText);
    } catch {
      return;
    }
    setBusy(true);
    setError(null);
    ipcCashRecordExpense(session.token, centimes, trimmedDescription, newCommandId())
      .then((movement) => {
        let formatted = movement.amountCentimes;
        try {
          formatted = formatMoney(movement.amountCentimes, locale);
        } catch {
          formatted = movement.amountCentimes;
        }
        setDone(formatted);
        void refresh();
        void ipcCashGetSessionReport(session.token, movement.cashSessionId).catch(() => undefined);
      })
      .catch((failure: unknown) => {
        const code = typeof failure === "string" ? failure : "INTERNAL_ERROR";
        setError(translateBackendError(code, locale));
      })
      .finally(() => {
        setBusy(false);
      });
  };

  if (done !== null) {
    return (
      <ModalShell title={t(locale, "cash.expenseTitle")} onClose={closeModal}>
        <div className="flex flex-col gap-3">
          <p className="rounded-lg bg-emerald-50 px-3 py-3 text-center text-base font-black text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
            {done}
          </p>
          <button
            type="button"
            onClick={closeModal}
            className="h-12 rounded-lg bg-accent-500 text-base font-black text-slate-950 hover:bg-amber-400"
          >
            {t(locale, "cash.confirm")}
          </button>
        </div>
      </ModalShell>
    );
  }

  return (
    <ModalShell title={t(locale, "cash.expenseTitle")} onClose={closeModal}>
      <div className="flex flex-col gap-3">
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold text-slate-500">{t(locale, "cash.expenseAmount")}</span>
          <input
            value={amountText}
            onChange={(event) => {
              setAmountText(event.target.value);
            }}
            inputMode="decimal"
            autoComplete="off"
            className="h-11 rounded-lg border border-slate-300 bg-white px-3 font-mono text-base font-black outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800"
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold text-slate-500">
            {t(locale, "cash.expenseDescription")}
          </span>
          <textarea
            value={description}
            onChange={(event) => {
              setDescription(event.target.value);
            }}
            rows={3}
            className="rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm font-bold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800"
          />
        </label>
        {error !== null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {error}
          </p>
        )}
        <div className="flex gap-2">
          <button
            type="button"
            onClick={closeModal}
            className="h-12 flex-1 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 dark:bg-slate-700"
          >
            {t(locale, "cash.cancel")}
          </button>
          <button
            type="button"
            onClick={confirm}
            disabled={!canConfirm}
            className="h-12 flex-[2] rounded-lg bg-accent-500 text-base font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
          >
            {t(locale, "cash.confirm")}
          </button>
        </div>
      </div>
    </ModalShell>
  );
}
