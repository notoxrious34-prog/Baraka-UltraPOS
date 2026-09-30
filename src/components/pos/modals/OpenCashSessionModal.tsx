/**
 * Open cash session modal: warehouse select + opening float (DZD).
 * Confirm disabled while the amount field is invalid or empty.
 */

import { useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { useCashSessionStore } from "@/stores/cash-session-store";
import { t, translateBackendError } from "@/lib/i18n";
import { newCommandId, parseMoneyToCentimes } from "@/lib/money";
import { ipcCashOpen } from "@/lib/ipc";
import { ModalShell } from "@/components/pos/modals/ModalShell";

function isValidAmount(text: string): boolean {
  if (text.trim() === "") {
    return false;
  }
  try {
    const centimes = BigInt(parseMoneyToCentimes(text));
    return centimes >= 0n;
  } catch {
    return false;
  }
}

export function OpenCashSessionModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const session = useSessionStore((state) => state.session);
  const warehouse = useSessionStore((state) => state.warehouse);
  const warehouses = useSessionStore((state) => state.warehouses);
  const setSession = useCashSessionStore((state) => state.setSession);
  const [warehouseId, setWarehouseId] = useState(() => warehouse?.id ?? "");
  const [amountText, setAmountText] = useState("0");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const valid = isValidAmount(amountText) && warehouseId.trim() !== "";
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
    ipcCashOpen(session.token, warehouseId, centimes, newCommandId())
      .then((opened) => {
        setSession(opened);
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

  return (
    <ModalShell title={t(locale, "cash.openTitle")} onClose={closeModal}>
      <div className="flex flex-col gap-3">
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold text-slate-500">{t(locale, "cash.warehouse")}</span>
          <select
            value={warehouseId}
            onChange={(event) => {
              setWarehouseId(event.target.value);
            }}
            className="h-11 rounded-lg border border-slate-300 bg-white px-3 text-sm font-bold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800"
          >
            <option value="">—</option>
            {warehouses
              .filter((candidate) => candidate.isActive)
              .map((candidate) => (
                <option key={candidate.id} value={candidate.id}>
                  {candidate.code} — {candidate.name}
                </option>
              ))}
          </select>
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold text-slate-500">{t(locale, "cash.openFloat")}</span>
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
