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

import { useEffect, useRef, useState, type ReactElement } from "react";
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

/**
 * State 3 UX guard predicate (Prototype 05, TASK-DSN-022): true when the
 * entered canonical amount strictly exceeds the backend-sourced balance
 * while the session role holds no overpayment grant. Pure BigInt
 * comparison — never throws (unparsable input reads as non-exceeding).
 */
export function isPaymentExceedingBalance(
  amountCentimes: string,
  balanceCentimes: string,
  allowOverpayment: boolean,
): boolean {
  if (allowOverpayment) {
    return false;
  }
  try {
    return BigInt(amountCentimes) > BigInt(balanceCentimes);
  } catch {
    return false;
  }
}

/** Amount validation outcome (i18n key on failure, UX-only guard). */
export type AmountValidation =
  | { readonly ok: true; readonly centimes: string }
  | { readonly ok: false; readonly key: "payment.invalidAmount" | "payment.vendeurRestricted" };

/**
 * Amount validation (DSN-073): the record gate arrives as a permission
 * boolean from `hasPermission("RECORD_CREDIT_PAYMENT")` — VENDEUR holds no
 * such permission (backend: Unauthorized), hence the restricted key.
 */
export function validatePaymentAmount(
  amountText: string,
  canRecordPayment: boolean,
): AmountValidation {
  if (!canRecordPayment) {
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
  const checked = validatePaymentAmount(args.amountText, true);
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
  const getSessionToken = useSessionStore((state) => state.getSessionToken);
  // Live permission reads (DSN-073): stable closures over `get()`, so the
  // gates stay observable under SSR/static markup. No role strings here.
  const hasPermission = useSessionStore((state) => state.hasPermission);
  const canRecordPayment = hasPermission("RECORD_CREDIT_PAYMENT");
  const overpaymentAllowed = hasPermission("MANAGE_CUSTOMER_CREDIT");
  const notify = usePosUiStore((state) => state.notify);
  const [amountText, setAmountText] = useState("");
  const [method, setMethod] = useState<PaymentMethodOption>("CASH");
  const [note, setNote] = useState("");
  const [allowOverpayment, setAllowOverpayment] = useState(false);
  const [busy, setBusy] = useState(false);
  const [errorCode, setErrorCode] = useState<string | null>(null);
  // State 4 receipt view (Prototype 05, TASK-DSN-022): set once on
  // successful submit; cleared on every fresh open (see reset below).
  const [completedPayment, setCompletedPayment] = useState<{
    readonly amountCentimes: string;
    readonly newBalanceCentimes: string;
  } | null>(null);
  // Synchronous in-flight lock (TASK-BRK-032 pattern): React state alone
  // cannot block same-tick double triggers, the ref is authoritative.
  const busyRef = useRef(false);

  // Fresh-open reset: the modal stays mounted across payments, so every
  // new opening clears the previous receipt view and draft input.
  useEffect(() => {
    if (isOpen) {
      setCompletedPayment(null);
      setAmountText("");
      setErrorCode(null);
    }
  }, [isOpen]);

  if (!isOpen) {
    return null;
  }

  const token = getSessionToken();
  const recordBlocked = !canRecordPayment;

  const validation = validatePaymentAmount(amountText, canRecordPayment);
  // State 3 guard: a non-privileged role may never submit an amount that
  // strictly exceeds the backend-sourced outstanding balance.
  const isExceedingBalance =
    validation.ok &&
    isPaymentExceedingBalance(
      validation.centimes,
      currentBalanceCentimes,
      overpaymentAllowed,
    );
  const exceedsWarning =
    isExceedingBalance && validation.ok
      ? t(locale, "payment.exceedsBalanceWarning")
          .replace("{amount}", formatMoney(validation.centimes, locale))
          .replace("{balance}", formatMoney(currentBalanceCentimes, locale))
      : null;
  const canSubmit =
    token !== null &&
    !busy &&
    !recordBlocked &&
    validation.ok &&
    !isExceedingBalance &&
    (overpaymentAllowed || !allowOverpayment);

  const submit = async (): Promise<void> => {
    if (busyRef.current) {
      return;
    }
    if (token === null || !canRecordPayment) {
      return;
    }
    const checked = validatePaymentAmount(amountText, canRecordPayment);
    if (!checked.ok) {
      setErrorCode(checked.key);
      return;
    }
    // Defense in depth: the submit button is disabled on exceed, but the
    // guard holds here too — the backend remains the final authority.
    if (
      isPaymentExceedingBalance(
        checked.centimes,
        currentBalanceCentimes,
        overpaymentAllowed,
      )
    ) {
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
      notify({
        title: t(locale, "payment.success").replace(
          "{balance}",
          formatMoney(result.new_balance_centimes, locale),
        ),
      });
      // State 4: show the سند قبض receipt view; the caller refresh
      // (onSuccess) and close run from the Fermer/إغلاق button instead.
      setCompletedPayment({
        amountCentimes: checked.centimes,
        newBalanceCentimes: result.new_balance_centimes,
      });
    } catch (failure: unknown) {
      setErrorCode(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  };

  // State 4 success receipt view (Prototype 05, TASK-DSN-022): flushes
  // the caller refresh exactly once, whether the cashier presses the
  // Fermer/إغلاق button or the shell close control.
  if (completedPayment !== null) {
    const closeCompleted = (): void => {
      onSuccess(completedPayment.newBalanceCentimes);
      onClose();
    };
    return (
      <ModalShell title={t(locale, "payment.title")} onClose={closeCompleted}>
        <div className="flex flex-col gap-3">
          <p className="flex items-center gap-2 rounded-md bg-emerald-50 px-3 py-2 text-sm font-black text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
            <svg
              className="h-5 w-5 shrink-0"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
              fill="none"
              aria-hidden="true"
            >
              <path d="M5 12l5 5L20 7" />
            </svg>
            <span>{t(locale, "payment.successTitle")}</span>
          </p>
          <div className="rounded-lg border border-slate-200 bg-slate-50 p-4 dark:border-slate-700 dark:bg-slate-800/60">
            <p className="text-sm font-black text-slate-800 dark:text-slate-100">
              {t(locale, "payment.receiptHeading")}
            </p>
            <p className="mt-1 truncate text-xs font-bold text-slate-600 dark:text-slate-300">
              {t(locale, "payment.customerLabel")} · {customerName}
            </p>
            <p className="mt-2 flex items-center justify-between text-xs font-bold text-slate-600 dark:text-slate-300">
              <span>{t(locale, "payment.amountCollectedLabel")}</span>
              <bdi className="font-mono font-black tabular-nums">
                {formatMoney(completedPayment.amountCentimes, locale)}
              </bdi>
            </p>
            <p className="mt-1 flex items-center justify-between text-xs font-bold text-slate-600 dark:text-slate-300">
              <span>{t(locale, "payment.newBalanceLabel")}</span>
              <bdi className="font-mono font-black tabular-nums">
                {formatMoney(completedPayment.newBalanceCentimes, locale)}
              </bdi>
            </p>
            <p className="mt-3 flex items-start gap-1.5 text-xs font-bold text-slate-500">
              <svg
                className="mt-0.5 h-4 w-4 shrink-0"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
                fill="none"
                aria-hidden="true"
              >
                <circle cx="12" cy="12" r="9" />
                <path d="M12 11v5M12 8v.01" />
              </svg>
              <span>{t(locale, "payment.printNote")}</span>
            </p>
          </div>
          <button
            type="button"
            onClick={closeCompleted}
            className="h-11 w-full rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400"
          >
            {t(locale, "payment.close")}
          </button>
        </div>
      </ModalShell>
    );
  }

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
            disabled={busy || recordBlocked}
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
                disabled={busy || recordBlocked}
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
            disabled={busy || recordBlocked}
            className="h-11 rounded-lg border border-slate-300 bg-white px-3 font-mono text-sm font-bold outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
          />
        </label>

        <label className="flex items-center gap-2 text-xs font-bold">
          <input
            type="checkbox"
            checked={allowOverpayment}
            disabled={busy || recordBlocked || !overpaymentAllowed}
            onChange={(event) => {
              setAllowOverpayment(event.target.checked);
            }}
          />
          <span className={overpaymentAllowed ? "" : "opacity-60"}>
            {t(locale, "payment.allowOverpayment")}
          </span>
        </label>
        {!overpaymentAllowed && !recordBlocked && (
          <p className="text-[11px] font-bold text-slate-500">
            {t(locale, "payment.overpaymentRestricted")}
          </p>
        )}
        {recordBlocked && (
          <p className="rounded-md bg-amber-50 px-3 py-2 text-sm font-bold text-amber-800 dark:bg-amber-900/30 dark:text-amber-200">
            {t(locale, "payment.vendeurRestricted")}
          </p>
        )}
        {!validation.ok && !recordBlocked && amountText.trim() !== "" && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {t(locale, validation.key)}
          </p>
        )}
        {exceedsWarning !== null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {exceedsWarning}
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
            title={!canRecordPayment ? t(locale, "customers.paymentReservedGuidance") : undefined}
            className="h-11 flex-[2] rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
          >
            {busy ? t(locale, "payment.submitting") : t(locale, "payment.submit")}
          </button>
        </div>
      </div>
    </ModalShell>
  );
}
