/**
 * Record credit payment (Versement) modal (TASK-BRK-037).
 *
 * Pure frontend wiring over the existing `credit_record_payment` IPC
 * command. Money travels exclusively as canonical centime strings
 * (BigInt comparisons only — no floats, no local balance arithmetic).
 * The backend remains the final authority; all client-side guards are
 * UX-only. The payment method selector is a UX label folded into the
 * opaque backend `reference` string (`TAG` or `TAG:user-note`) because
 * the customer credit domain carries no method column.
 */

import { useRef, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { formatMoney, parseMoneyToCentimes } from "@/lib/money";
import { ipcRecordCreditPayment } from "@/lib/ipc";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { ModalShell } from "@/components/pos/modals/ModalShell";

/** Payment method options (UX labels; folded into `reference` on submit). */
export type PaymentMethodOption = "CASH" | "CHEQUE" | "TRANSFER";

export const PAYMENT_METHODS: readonly PaymentMethodOption[] = [
  "CASH",
  "CHEQUE",
  "TRANSFER",
];

/** Ledger-searchable tag per method (backend stores `reference` opaquely). */
export function methodTag(method: PaymentMethodOption): string {
  switch (method) {
    case "CASH":
      return "ESPECES";
    case "CHEQUE":
      return "CHEQUE";
    case "TRANSFER":
      return "VIREMENT";
  }
}

/**
 * Build the backend `reference` payload: always carries the method tag so
 * the ledger line stays attributable; the cashier note is appended raw.
 */
export function buildPaymentReference(
  method: PaymentMethodOption,
  note: string,
): string {
  const tag = methodTag(method);
  const trimmed = note.trim();
  return trimmed === "" ? tag : `${tag}:${trimmed}`;
}

/** VENDEUR holds no RecordCreditPayment permission (backend: Unauthorized). */
export function isVendeur(role: string): boolean {
  return role === "VENDEUR";
}

/** Overpayment (negative balance) needs ManageCustomerCredit: GERANT/ADMIN. */
export function canUseOverpayment(role: string): boolean {
  return role === "GERANT" || role === "ADMIN";
}

/** Amount validation outcome (i18n key on failure, UX-only guard). */
export type AmountValidation =
  | { readonly ok: true; readonly centimes: string }
  | { readonly ok: false; readonly key: "payment.invalidAmount" | "payment.vendeurRestricted" };

export function validatePaymentAmount(
  amountText: string,
  role: string,
): AmountValidation {
  if (isVendeur(role)) {
    return { ok: false, key: "payment.vendeurRestricted" };
  }
  let centimes = "0";
  try {
    centimes = parseMoneyToCentimes(amountText);
  } catch {
    return { ok: false, key: "payment.invalidAmount" };
  }
  if (BigInt(centimes) <= 0n) {
    return { ok: false, key: "payment.invalidAmount" };
  }
  return { ok: true, centimes };
}

/** Exact backend wire shape for `credit_record_payment` (snake_case keys). */
export interface RecordPaymentWire {
  readonly customer_id: string;
  readonly amount_centimes: string;
  readonly reference: string | null;
  readonly allow_overpayment: boolean;
}

export function buildRecordPaymentWire(args: {
  customerId: string;
  amountText: string;
  method: PaymentMethodOption;
  note: string;
  allowOverpayment: boolean;
}): RecordPaymentWire {
  const checked = validatePaymentAmount(args.amountText, "CAISSIER");
  if (!checked.ok) {
    throw new Error(checked.key);
  }
  const trimmedNote = args.note.trim();
  return {
    customer_id: args.customerId,
    amount_centimes: checked.centimes,
    reference:
      trimmedNote === "" && args.method === "CASH"
        ? null
        : buildPaymentReference(args.method, args.note),
    allow_overpayment: args.allowOverpayment,
  };
}

export interface RecordPaymentModalProps {
  readonly customerId: string;
  readonly customerName: string;
  /** Backend-sourced outstanding balance (canonical centime string). */
  readonly currentBalanceCentimes: string;
  readonly isOpen: boolean;
  readonly onClose: () => void;
  readonly onSuccess: (newBalanceCentimes: string) => void;
}

export function RecordPaymentModal({
  customerId,
  customerName,
  currentBalanceCentimes,
  isOpen,
  onClose,
  onSuccess,
}: RecordPaymentModalProps): ReactElement | null {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const setBanner = usePosUiStore((state) => state.setBanner);
  const [amountText, setAmountText] = useState("");
  const [method, setMethod] = useState<PaymentMethodOption>("CASH");
  const [note, setNote] = useState("");
  const [allowOverpayment, setAllowOverpayment] = useState(false);
  const [busy, setBusy] = useState(false);
  const [errorCode, setErrorCode] = useState<string | null>(null);
  // Synchronous in-flight lock (TASK-BRK-032 pattern): React state alone
  // cannot block same-tick double triggers, the ref is authoritative.
  const busyRef = useRef(false);

  if (!isOpen) {
    return null;
  }

  const role = session !== null ? session.role : "";
  const token = session !== null ? session.token : null;
  const vendeurBlocked = isVendeur(role);
  const overpaymentAllowed = canUseOverpayment(role);

  const validation = validatePaymentAmount(amountText, role);
  const canSubmit =
    token !== null &&
    !busy &&
    !vendeurBlocked &&
    validation.ok &&
    (overpaymentAllowed || !allowOverpayment);

  const submit = async (): Promise<void> => {
    if (busyRef.current) {
      return;
    }
    if (token === null || vendeurBlocked) {
      return;
    }
    const checked = validatePaymentAmount(amountText, role);
    if (!checked.ok) {
      setErrorCode(checked.key);
      return;
    }
    busyRef.current = true;
    setBusy(true);
    setErrorCode(null);
    try {
      const result = await ipcRecordCreditPayment(
        token,
        customerId,
        checked.centimes,
        buildPaymentReference(method, note),
        overpaymentAllowed && allowOverpayment,
      );
      setBanner(
        t(locale, "payment.success").replace(
          "{balance}",
          formatMoney(result.new_balance_centimes, locale),
        ),
      );
      onSuccess(result.new_balance_centimes);
      onClose();
    } catch (failure: unknown) {
      setErrorCode(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  };

  return (
    <ModalShell title={t(locale, "payment.title")} onClose={onClose}>
      <div className="flex flex-col gap-3">
        <div className="rounded-lg bg-slate-100 px-3 py-2 dark:bg-slate-800">
          <p className="truncate text-sm font-extrabold">{customerName}</p>
          <p className="font-mono text-xs opacity-70">
            {t(locale, "payment.currentBalance")} ·{" "}
            {formatMoney(currentBalanceCentimes, locale)}
          </p>
        </div>

        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold text-slate-500">
            {t(locale, "payment.amountLabel")}
          </span>
          <input
            value={amountText}
            onChange={(event) => {
              setAmountText(event.target.value);
            }}
            inputMode="decimal"
            autoComplete="off"
            dir="ltr"
            disabled={busy || vendeurBlocked}
            className="h-11 rounded-lg border border-slate-300 bg-white px-3 font-mono text-base font-black outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
          />
        </label>

        <div className="flex flex-col gap-1">
          <span className="text-xs font-bold text-slate-500">
            {t(locale, "payment.methodLabel")}
          </span>
          <div className="grid grid-cols-3 gap-2">
            {PAYMENT_METHODS.map((option) => (
              <button
                key={option}
                type="button"
                disabled={busy || vendeurBlocked}
                onClick={() => {
                  setMethod(option);
                }}
                className={`rounded-lg px-2 py-2 text-xs font-black disabled:opacity-50 ${
                  method === option
                    ? "bg-primary-600 text-white"
                    : "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-300"
                }`}
              >
                {t(
                  locale,
                  option === "CASH"
                    ? "payment.methodCash"
                    : option === "CHEQUE"
                      ? "payment.methodCheque"
                      : "payment.methodTransfer",
                )}
              </button>
            ))}
          </div>
        </div>

        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold text-slate-500">
            {t(locale, "payment.referenceLabel")}
          </span>
          <input
            value={note}
            onChange={(event) => {
              setNote(event.target.value);
            }}
            autoComplete="off"
            dir="ltr"
            disabled={busy || vendeurBlocked}
            className="h-11 rounded-lg border border-slate-300 bg-white px-3 font-mono text-sm font-bold outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
          />
        </label>

        <label className="flex items-center gap-2 text-xs font-bold">
          <input
            type="checkbox"
            checked={allowOverpayment}
            disabled={busy || vendeurBlocked || !overpaymentAllowed}
            onChange={(event) => {
              setAllowOverpayment(event.target.checked);
            }}
          />
          <span className={overpaymentAllowed ? "" : "opacity-60"}>
            {t(locale, "payment.allowOverpayment")}
          </span>
        </label>
        {!overpaymentAllowed && !vendeurBlocked && (
          <p className="text-[11px] font-bold text-slate-500">
            {t(locale, "payment.overpaymentRestricted")}
          </p>
        )}
        {vendeurBlocked && (
          <p className="rounded-md bg-amber-50 px-3 py-2 text-sm font-bold text-amber-800 dark:bg-amber-900/30 dark:text-amber-200">
            {t(locale, "payment.vendeurRestricted")}
          </p>
        )}
        {!validation.ok && !vendeurBlocked && amountText.trim() !== "" && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {t(locale, validation.key)}
          </p>
        )}
        {errorCode !== null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {translateBackendError(errorCode, locale)}
          </p>
        )}

        <div className="flex gap-2">
          <button
            type="button"
            onClick={onClose}
            disabled={busy}
            className="h-11 flex-1 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:text-slate-100"
          >
            {t(locale, "payment.cancel")}
          </button>
          <button
            type="button"
            onClick={() => {
              void submit();
            }}
            disabled={!canSubmit}
            className="h-11 flex-[2] rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
          >
            {busy ? t(locale, "payment.submitting") : t(locale, "payment.submit")}
          </button>
        </div>
      </div>
    </ModalShell>
  );
}
