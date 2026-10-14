/**
 * Customer create/edit form modal (TASK-DSN-027): shared validation and
 * string-only centime conversion (parseMoneyToCentimes — zero floats),
 * following the CustomerSelectModal creation precedent.
 */

import { useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { ipcCreateCustomer, ipcGetCustomer, ipcUpdateCustomer } from "@/lib/ipc";
import { parseMoneyToCentimes } from "@/lib/money";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import type { CustomerDetail, CustomerTier, Locale } from "@/types/pos";
import { isValidCustomerName } from "@/features/customers/customer-helpers";

type CreditPolicy = "DISABLED" | "LIMITED" | "UNLIMITED";

export type CustomerFormMode =
  | { readonly kind: "create" }
  | { readonly kind: "edit"; readonly detail: CustomerDetail };

interface CustomerFormModalProps {
  locale: Locale;
  token: string;
  role: string;
  mode: CustomerFormMode;
  onClose: () => void;
  onSaved: (detail: CustomerDetail) => void;
}

function canManageCredit(role: string): boolean {
  return role === "GERANT" || role === "ADMIN";
}

const inputClass =
  "h-11 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm font-semibold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800";
const labelClass = "text-xs font-bold text-slate-500";

export function CustomerFormModal(props: CustomerFormModalProps): ReactElement {
  const { locale, mode } = props;
  const editing = mode.kind === "edit" ? mode.detail : null;
  const [name, setName] = useState(editing?.name ?? "");
  const [phone, setPhone] = useState(editing?.phone ?? "");
  const [address, setAddress] = useState(editing?.address ?? "");
  const [policy, setPolicy] = useState<CreditPolicy>(editing?.credit_policy ?? "DISABLED");
  const [tier, setTier] = useState<CustomerTier>(editing?.customer_tier ?? "retail");
  const [limitText, setLimitText] = useState(editing?.credit_limit_centimes ?? "");
  const [active, setActive] = useState(editing?.is_active ?? true);
  const [formError, setFormError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const submit = (): void => {
    if (saving) {
      return;
    }
    if (!isValidCustomerName(name)) {
      setFormError(t(locale, "customers.formNameError"));
      return;
    }
    let limit: string | null = null;
    if (policy === "LIMITED") {
      if (!canManageCredit(props.role)) {
        setFormError(translateBackendError("UNAUTHORIZED", locale));
        return;
      }
      if (limitText.trim() === "") {
        setFormError(t(locale, "customers.formLimitError"));
        return;
      }
      try {
        limit = parseMoneyToCentimes(limitText);
      } catch {
        setFormError(t(locale, "customers.formLimitError"));
        return;
      }
    }
    setSaving(true);
    setFormError(null);
    const done = (detail: CustomerDetail): void => {
      setSaving(false);
      props.onSaved(detail);
    };
    const failed = (failure: unknown): void => {
      setSaving(false);
      setFormError(
        translateBackendError(typeof failure === "string" ? failure : "INTERNAL_ERROR", locale),
      );
    };
    if (editing !== null) {
      void ipcUpdateCustomer(props.token, {
        customer_id: editing.id,
        name: name.trim(),
        phone: phone.trim() === "" ? null : phone.trim(),
        address: address.trim() === "" ? null : address.trim(),
        credit_policy: policy,
        credit_limit_centimes: limit,
        customer_tier: tier,
        is_active: active,
      })
        .then(done)
        .catch(failed);
    } else {
      void ipcCreateCustomer(props.token, {
        name: name.trim(),
        phone: phone.trim() === "" ? null : phone.trim(),
        address: address.trim() === "" ? null : address.trim(),
        credit_policy: policy,
        credit_limit_centimes: limit,
        customer_tier: tier,
      })
        .then((id) => ipcGetCustomer(props.token, id).then(done).catch(failed))
        .catch(failed);
    }
  };

  return (
    <ModalShell
      title={t(locale, editing !== null ? "customers.formEdit" : "customers.formCreate")}
      onClose={props.onClose}
    >
      <form
        className="flex flex-col gap-3"
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <label className="flex flex-col gap-1">
          <span className={labelClass}>{t(locale, "customers.formName")}</span>
          <input value={name} onChange={(e) => setName(e.target.value)} className={inputClass} />
        </label>
        <label className="flex flex-col gap-1">
          <span className={labelClass}>{t(locale, "customers.formPhone")}</span>
          <input
            value={phone}
            onChange={(e) => setPhone(e.target.value)}
            className={inputClass}
            inputMode="tel"
            dir="ltr"
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className={labelClass}>{t(locale, "customers.formAddress")}</span>
          <input value={address} onChange={(e) => setAddress(e.target.value)} className={inputClass} />
        </label>
        <label className="flex flex-col gap-1">
          <span className={labelClass}>{t(locale, "customers.formPolicy")}</span>
          <select
            value={policy}
            onChange={(e) => setPolicy(e.target.value as CreditPolicy)}
            className={inputClass}
          >
            <option value="DISABLED">DISABLED</option>
            <option value="LIMITED">LIMITED</option>
            <option value="UNLIMITED">UNLIMITED</option>
          </select>
        </label>
        {policy === "LIMITED" && (
          <label className="flex flex-col gap-1">
            <span className={labelClass}>{t(locale, "customers.formLimit")}</span>
            <input
              value={limitText}
              onChange={(e) => setLimitText(e.target.value)}
              className={inputClass}
              inputMode="decimal"
              dir="ltr"
            />
          </label>
        )}
        <div className="flex flex-col gap-1">
          <span className={labelClass}>{t(locale, "customers.formTier")}</span>
          <div className="grid grid-cols-3 gap-2">
            {(
              [
                { id: "retail", label: t(locale, "tier_retail") },
                { id: "artisan", label: t(locale, "tier_artisan") },
                { id: "wholesale", label: t(locale, "tier_wholesale") },
              ] as const
            ).map((option) => (
              <button
                key={option.id}
                type="button"
                onClick={() => {
                  setTier(option.id);
                }}
                className={`rounded-lg px-2 py-2 text-xs font-black ${
                  tier === option.id
                    ? "bg-amber-500 text-slate-950"
                    : "bg-slate-200 text-slate-600 dark:bg-slate-700 dark:text-slate-300"
                }`}
              >
                {option.label}
              </button>
            ))}
          </div>
        </div>
        {editing !== null && (
          <label className="flex items-center gap-2 text-sm font-bold">
            <input
              type="checkbox"
              checked={active}
              onChange={(e) => setActive(e.target.checked)}
            />
            {t(locale, "customers.formActive")}
          </label>
        )}
        {formError !== null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-xs font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {formError}
          </p>
        )}
        <div className="flex gap-2">
          <button
            type="submit"
            disabled={saving}
            className="h-11 flex-1 rounded-lg bg-accent-500 text-sm font-black text-slate-950 disabled:opacity-50"
          >
            {saving ? t(locale, "customers.formSaving") : t(locale, "customers.formSave")}
          </button>
          <button
            type="button"
            onClick={props.onClose}
            className="h-11 rounded-lg bg-slate-200 px-4 text-sm font-bold dark:bg-slate-700"
          >
            {t(locale, "customers.formCancel")}
          </button>
        </div>
      </form>
    </ModalShell>
  );
}
