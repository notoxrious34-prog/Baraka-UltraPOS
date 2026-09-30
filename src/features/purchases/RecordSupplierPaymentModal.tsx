/**
 * Record-supplier-payment modal (ADR-030): CASH requires an OPEN session
 * and writes one negative SUPPLIER_PAYMENT_OUT drawer movement; OTHER
 * (transfer/cheque) never touches the drawer. Bilingual errors only.
 */

import { useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { formatMoney, newCommandId, parseMoneyToCentimes } from "@/lib/money";
import { ipcRecordSupplierPayment } from "@/lib/ipc";
import type { SupplierPaymentOutcome, SupplierView } from "@/types/supplier";

interface Props {
  token: string;
  supplier: SupplierView | null;
  onClose: () => void;
}

export function RecordSupplierPaymentModal({ token, supplier, onClose }: Props): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const [pasted, setPasted] = useState("");
  const [amountText, setAmountText] = useState("");
  const [method, setMethod] = useState<"CASH" | "OTHER">("CASH");
  const [description, setDescription] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<SupplierPaymentOutcome | null>(null);

  const resolvedSupplier = supplier?.id ?? pasted.trim();

  const submit = async (): Promise<void> => {
    if (busy) {
      return;
    }
    if (resolvedSupplier === "") {
      setError("SUPPLIER_NOT_FOUND");
      return;
    }
    let amountCentimes = "0";
    try {
      amountCentimes = parseMoneyToCentimes(amountText);
    } catch {
      setError("SUPPLIER_PAYMENT_INVALID_AMOUNT");
      return;
    }
    if (BigInt(amountCentimes) <= 0n) {
      setError("SUPPLIER_PAYMENT_INVALID_AMOUNT");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const outcome = await ipcRecordSupplierPayment(
        token,
        newCommandId(),
        resolvedSupplier,
        amountCentimes,
        method,
        description.trim() === "" ? null : description.trim(),
      );
      setDone(outcome);
    } catch (failure: unknown) {
      setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 p-4">
      <div className="flex max-h-full w-full max-w-md flex-col gap-3 overflow-y-auto rounded-xl bg-white p-4 dark:bg-slate-900">
        <h3 className="text-base font-black">{t(locale, "pur.payment")}</h3>
        {supplier === null ? (
          <input
            value={pasted}
            onChange={(event) => {
              setPasted(event.target.value);
            }}
            placeholder={`${t(locale, "sup.idLabel")} — ${t(locale, "pur.optionalId")}`}
            className="h-10 rounded-lg border border-slate-300 bg-white px-2 font-mono text-xs font-bold outline-none dark:border-slate-700 dark:bg-slate-800"
          />
        ) : (
          <p className="text-sm font-black">{supplier.name}</p>
        )}
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold text-slate-500">{t(locale, "pur.amount")}</span>
          <input
            value={amountText}
            onChange={(event) => {
              setAmountText(event.target.value);
            }}
            inputMode="decimal"
            className="h-10 rounded-lg border border-slate-300 bg-white px-3 font-mono text-base font-black outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800"
          />
        </label>
        <div className="flex flex-col gap-1">
          <span className="text-xs font-bold text-slate-500">{t(locale, "pur.method")}</span>
          <div className="flex gap-2">
            {(["CASH", "OTHER"] as const).map((option) => (
              <button
                key={option}
                type="button"
                onClick={() => {
                  setMethod(option);
                }}
                className={`flex-1 rounded-lg px-3 py-2 text-sm font-black ${
                  method === option
                    ? "bg-primary-600 text-white"
                    : "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-200"
                }`}
              >
                {t(locale, option === "CASH" ? "pur.cash" : "pur.other")}
              </button>
            ))}
          </div>
        </div>
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold text-slate-500">{t(locale, "pur.description")}</span>
          <input
            value={description}
            onChange={(event) => {
              setDescription(event.target.value);
            }}
            className="h-10 rounded-lg border border-slate-300 bg-white px-3 text-sm font-bold outline-none dark:border-slate-700 dark:bg-slate-800"
          />
        </label>
        {error !== null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {translateBackendError(error, locale)}
          </p>
        )}
        {done !== null && (
          <p className="rounded-md bg-emerald-50 px-3 py-2 text-sm font-bold text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
            {t(locale, "pur.paid")} · {t(locale, "sup.balance")}{" "}
            {formatMoney(done.newBalanceCentimes, locale)}
            {done.cashMovementId !== null && ` · ${t(locale, "pur.cash")}`}
          </p>
        )}
        <div className="flex gap-2">
          <button
            type="button"
            onClick={onClose}
            className="h-11 flex-1 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-100"
          >
            {t(locale, "common.cancel")}
          </button>
          <button
            type="button"
            onClick={() => {
              void submit();
            }}
            disabled={busy || amountText.trim() === "" || resolvedSupplier === ""}
            className="h-11 flex-[2] rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
          >
            {busy ? t(locale, "pur.submitting") : t(locale, "pur.submit")}
          </button>
        </div>
      </div>
    </div>
  );
}
