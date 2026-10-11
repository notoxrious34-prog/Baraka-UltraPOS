/**
 * Warehouse creation modal (TASK-DSN-029): ADMIN-gated code/name form with
 * inline validation, following the CustomerFormModal precedent. Code is
 * uppercased at the IPC boundary (ipcCreateWarehouse trims + uppercases).
 */

import { useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { ipcCreateWarehouse } from "@/lib/ipc";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import type { Locale, WarehouseInfo } from "@/types/pos";
import { isValidWarehouseCode, isValidWarehouseName } from "@/features/stock/stock-helpers";

interface WarehouseCreateModalProps {
  locale: Locale;
  token: string;
  onClose: () => void;
  onSaved: (warehouse: WarehouseInfo) => void;
}

const inputClass =
  "h-11 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm font-semibold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800";
const labelClass = "text-xs font-bold text-slate-500";

export function WarehouseCreateModal(props: WarehouseCreateModalProps): ReactElement {
  const { locale } = props;
  const [code, setCode] = useState("");
  const [name, setName] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const submit = (): void => {
    if (saving) {
      return;
    }
    if (!isValidWarehouseCode(code)) {
      setFormError(t(locale, "stock.formCodeError"));
      return;
    }
    if (!isValidWarehouseName(name)) {
      setFormError(t(locale, "stock.formNameError"));
      return;
    }
    setSaving(true);
    setFormError(null);
    ipcCreateWarehouse(props.token, code, name)
      .then((warehouse) => {
        setSaving(false);
        props.onSaved(warehouse);
      })
      .catch((failure: unknown) => {
        setSaving(false);
        setFormError(
          translateBackendError(typeof failure === "string" ? failure : "INTERNAL_ERROR", locale),
        );
      });
  };

  return (
    <ModalShell title={t(locale, "stock.formTitle")} onClose={props.onClose}>
      <form
        className="flex flex-col gap-3"
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <label className="flex flex-col gap-1">
          <span className={labelClass}>{t(locale, "stock.formCode")}</span>
          <input
            value={code}
            onChange={(event) => setCode(event.target.value)}
            className={inputClass}
            dir="ltr"
            autoComplete="off"
            maxLength={20}
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className={labelClass}>{t(locale, "stock.formName")}</span>
          <input
            value={name}
            onChange={(event) => setName(event.target.value)}
            className={inputClass}
            autoComplete="off"
            maxLength={100}
          />
        </label>
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
            {saving ? t(locale, "stock.formSaving") : t(locale, "stock.formSave")}
          </button>
          <button
            type="button"
            onClick={props.onClose}
            className="h-11 rounded-lg bg-slate-200 px-4 text-sm font-bold dark:bg-slate-700"
          >
            {t(locale, "stock.formCancel")}
          </button>
        </div>
      </form>
    </ModalShell>
  );
}
