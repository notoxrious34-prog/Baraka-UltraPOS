/**
 * Customer dossier detail pane (TASK-DSN-027): identity header, financial
 * HUD cards (solde dû / plafond / disponible), edit entry point, and the
 * ledger statement section. Teaching empty state when nothing is selected.
 */

import type { ReactElement } from "react";
import { t } from "@/lib/i18n";
import { formatMoney } from "@/lib/money";
import type { CustomerDetail, Locale } from "@/types/pos";
import { CustomerStatementTable } from "@/features/customers/CustomerStatementTable";
import { availableCredit, isDebtor } from "@/features/customers/customer-helpers";

interface CustomerDossierPaneProps {
  locale: Locale;
  token: string;
  detail: CustomerDetail | null;
  onEdit: () => void;
}

function fmtAmount(centimes: string, locale: Locale): string {
  try {
    return formatMoney(centimes, locale);
  } catch {
    return "…";
  }
}

function HudCard(props: {
  label: string;
  value: string;
  alert: boolean;
}): ReactElement {
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-1 rounded-lg border border-slate-200 p-3 dark:border-slate-700">
      <span className="text-[11px] font-bold text-slate-500">{props.label}</span>
      <bdi
        dir="ltr"
        className={`font-mono text-base font-black tabular-nums ${
          props.alert ? "text-amber-600 dark:text-amber-400" : ""
        }`}
      >
        {props.value}
      </bdi>
    </div>
  );
}

export function CustomerDossierPane(props: CustomerDossierPaneProps): ReactElement {
  const { locale, detail } = props;
  if (detail === null) {
    return (
      <section className="flex min-h-0 flex-1 items-center justify-center p-8">
        <p className="max-w-md text-center text-sm font-medium text-slate-500">
          {t(locale, "customers.selectHint")}
        </p>
      </section>
    );
  }
  const debtor = isDebtor(detail.current_balance_centimes);
  const remaining = availableCredit(detail);
  const limitText =
    detail.credit_policy === "UNLIMITED"
      ? t(locale, "customers.unlimited")
      : detail.credit_policy === "DISABLED"
        ? t(locale, "customers.blocked")
        : detail.credit_limit_centimes !== null
          ? fmtAmount(detail.credit_limit_centimes, locale)
          : "—";
  return (
    <section className="flex min-h-0 min-w-0 flex-1 flex-col gap-4 overflow-y-auto p-4">
      <div className="flex shrink-0 flex-wrap items-start gap-3">
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-lg font-black">{detail.name}</h2>
          <p className="text-xs text-slate-500">
            <bdi dir="ltr">{detail.phone ?? "—"}</bdi>
            {detail.address !== null && detail.address !== "" && ` · ${detail.address}`}
          </p>
          {!detail.is_active && (
            <span className="mt-1 inline-block rounded-full bg-slate-300 px-2 py-0.5 text-[10px] font-black text-slate-700 dark:bg-slate-700 dark:text-slate-200">
              ✕ {t(locale, "customers.filterInactive")}
            </span>
          )}
        </div>
        <button
          type="button"
          onClick={props.onEdit}
          className="h-10 shrink-0 rounded-lg bg-primary-900 px-4 text-sm font-bold text-white hover:bg-primary-800 dark:bg-primary-700"
        >
          {t(locale, "customers.edit")}
        </button>
      </div>
      <div className="flex shrink-0 flex-wrap gap-2">
        <HudCard
          label={t(locale, "customers.balanceDue")}
          value={fmtAmount(detail.current_balance_centimes, locale)}
          alert={debtor}
        />
        <HudCard label={t(locale, "customers.creditLimit")} value={limitText} alert={false} />
        <HudCard
          label={t(locale, "customers.availableCredit")}
          value={remaining !== null ? fmtAmount(remaining, locale) : "—"}
          alert={false}
        />
      </div>
      <CustomerStatementTable locale={locale} token={props.token} customerId={detail.id} />
    </section>
  );
}
