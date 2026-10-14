/**
 * Customer credit HUD: guest pill, unlimited badge, or a Green/Amber/Red
 * gauge driven by pure-BigInt basis points (no floats anywhere).
 */

import type { ReactElement } from "react";
import { useState } from "react";
import { useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { usePaymentModalStore } from "@/stores/payment-modal-store";
import { t, translateBackendError } from "@/lib/i18n";
import { creditUsageBp, formatMoney } from "@/lib/money";
import { ipcGetCustomer } from "@/lib/ipc";
import { CustomerTierBadge } from "@/components/pos/CustomerTierBadge";

export function CreditHud(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const customer = useCartStore((state) => state.customer);
  const openModal = usePosUiStore((state) => state.openModal);
  const session = useSessionStore((state) => state.session);
  const openPayment = usePaymentModalStore((state) => state.openPayment);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const open = (): void => {
    openModal("customer");
  };

  const openVersement = (): void => {
    if (customer.kind !== "named" || session === null || loading) {
      return;
    }
    const customerId = customer.id;
    setLoading(true);
    setError(null);
    ipcGetCustomer(session.token, customerId)
      .then((detail) => {
        openPayment({
          customerId: detail.id,
          customerName: detail.name,
          balanceCentimes: detail.current_balance_centimes,
          returnTo: null,
        });
        openModal("payment");
      })
      .catch((failure: unknown) => {
        setError(
          translateBackendError(
            typeof failure === "string" ? failure : "INTERNAL_ERROR",
            usePosUiStore.getState().locale,
          ),
        );
      })
      .finally(() => {
        setLoading(false);
      });
  };

  if (customer.kind === "guest") {
    return (
      <button
        type="button"
        onClick={open}
        className="flex w-full items-center justify-between rounded-lg bg-slate-200 px-3 py-2 text-sm font-bold text-slate-700 hover:bg-slate-300 dark:bg-slate-800 dark:text-slate-200"
      >
        <span>{t(locale, "cart.guest")}</span>
        <span className="font-mono text-[11px] opacity-70">F2</span>
      </button>
    );
  }

  if (customer.policy === "UNLIMITED") {
    return (
      <div className="flex w-full flex-col gap-1">
        <button
          type="button"
          onClick={open}
          className="flex w-full items-center justify-between rounded-lg bg-emerald-700 px-3 py-2 text-sm font-bold text-white"
        >
          <span className="flex min-w-0 items-center gap-1.5">
            <span className="truncate">{customer.name}</span>
            <CustomerTierBadge tier={customer.tier} locale={locale} />
          </span>
          <span className="shrink-0 rounded-full bg-white/20 px-2 py-0.5 text-[11px]">
            {t(locale, "credit.unlimited")}
          </span>
        </button>
        <VersementButton
          locale={locale}
          loading={loading}
          onOpen={openVersement}
          error={error}
        />
      </div>
    );
  }

  if (customer.policy === "DISABLED" || customer.limitCentimes === null || customer.balanceCentimes === null) {
    return (
      <button
        type="button"
        onClick={open}
        className="flex w-full flex-col gap-1 rounded-lg bg-amber-100 px-3 py-2 text-start text-sm font-bold text-amber-900 dark:bg-amber-900/40 dark:text-amber-200"
      >
        <span className="flex items-center gap-1.5">
          <span className="truncate">{customer.name}</span>
          <CustomerTierBadge tier={customer.tier} locale={locale} />
        </span>
        <span className="text-[11px] font-semibold opacity-80">{t(locale, "credit.pending")}</span>
      </button>
    );
  }

  let usage: bigint | null = null;
  try {
    usage = creditUsageBp(customer.balanceCentimes, customer.limitCentimes);
  } catch {
    usage = null;
  }
  if (usage === null) {
    return (
      <div className="flex w-full flex-col gap-1">
        <button
          type="button"
          onClick={open}
          className="w-full rounded-lg bg-slate-200 px-3 py-2 text-start text-sm font-bold dark:bg-slate-800 dark:text-slate-200"
        >
          <span className="flex items-center gap-1.5">
            <span className="truncate">{customer.name}</span>
            <CustomerTierBadge tier={customer.tier} locale={locale} />
          </span>
        </button>
        <VersementButton
          locale={locale}
          loading={loading}
          onOpen={openVersement}
          error={error}
        />
      </div>
    );
  }

  const blocked = usage > 10_000n;
  const barTone = blocked ? "bg-red-600" : usage > 7_500n ? "bg-amber-500" : "bg-emerald-600";
  const shellTone = blocked
    ? "bg-red-100 text-red-900 dark:bg-red-900/40 dark:text-red-200"
    : "bg-slate-100 text-slate-800 dark:bg-slate-800 dark:text-slate-100";
  const widthPct = usage > 10_000n ? 100 : Number(usage / 100n);

  return (
    <div className="flex w-full flex-col gap-1">
      <div className={`flex w-full flex-col gap-1 rounded-lg px-3 py-2 ${shellTone}`}>
        <button type="button" onClick={open} className="flex items-center justify-between gap-2 text-start">
          <span className="flex min-w-0 items-center gap-1.5">
            <span className="truncate text-sm font-extrabold">{customer.name}</span>
            <CustomerTierBadge tier={customer.tier} locale={locale} />
          </span>
          <span className="shrink-0 font-mono text-[11px] font-bold">
            {formatMoney(customer.balanceCentimes, locale)} / {formatMoney(customer.limitCentimes, locale)}
          </span>
        </button>
        <div className="h-2 w-full overflow-hidden rounded-full bg-black/15">
          <div className={`h-full rounded-full ${barTone}`} style={{ width: `${widthPct}%` }} />
        </div>
        {blocked && (
          <p className="text-[11px] font-black">{t(locale, "credit.blocked")}</p>
        )}
      </div>
      <VersementButton
        locale={locale}
        loading={loading}
        onOpen={openVersement}
        error={error}
      />
    </div>
  );
}

function VersementButton({
  locale,
  loading,
  onOpen,
  error,
}: {
  locale: "ar-DZ" | "fr-DZ";
  loading: boolean;
  onOpen: () => void;
  error: string | null;
}): ReactElement {
  return (
    <div className="flex w-full flex-col gap-1">
      <button
        type="button"
        onClick={onOpen}
        disabled={loading}
        className="flex w-full items-center justify-between rounded-lg bg-accent-500 px-3 py-2 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-50"
      >
        <span>{t(locale, "payment.openButton")}</span>
        <span className="font-mono text-[11px] opacity-70">{loading ? "…" : "د.ج"}</span>
      </button>
      {error !== null && (
        <p className="rounded-md bg-red-50 px-3 py-1 text-[11px] font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {error}
        </p>
      )}
    </div>
  );
}
