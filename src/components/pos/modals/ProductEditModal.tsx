/**
 * Product edit modal (TASK-BRK-050): full-state product update.
 *
 * SKU is display-only (backend-immutable); the untouched unit set is
 * echoed back with the edited base-unit prices applied. All money parses
 * via parseMoneyToCentimes and formats via formatMoney — zero floats.
 * Backend errors surface through translateBackendError; the backend is
 * final (both names required, unit-set invariants enforced server-side).
 */

import { useEffect, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import {
  formatMargin,
  useProductEditStore,
  validateProductEditForm,
} from "@/stores/product-edit-store";
import { t, translateBackendError } from "@/lib/i18n";
import { ModalShell } from "@/components/pos/modals/ModalShell";

export function ProductEditModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const s = useProductEditStore();

  const validation = validateProductEditForm(s.form);
  const canSubmit =
    !s.isSubmitting && s.successMessage === null && validation.ok;

  // Capture-phase Escape guard while dispatching (CheckoutModal precedent);
  // Enter submits from single-line inputs when the form is valid.
  useEffect(() => {
    if (!s.isSubmitting) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape" || event.key === "Enter" || event.key === "F12") {
        event.preventDefault();
        event.stopImmediatePropagation();
      }
    };
    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => {
      window.removeEventListener("keydown", onKeyDown, { capture: true });
    };
  }, [s.isSubmitting]);

  const close = (): void => {
    if (s.isSubmitting) {
      return;
    }
    s.closeModal();
    closeModal();
  };

  const inputClass =
    "h-11 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm font-semibold outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800";

  return (
    <ModalShell title={t(locale, "editProductTitle")} onClose={close}>
      <div className="flex flex-col gap-3">
        <div className="flex items-center gap-2">
          <span className="rounded-md bg-slate-200 px-2 py-1 font-mono text-xs font-black dark:bg-slate-700">
            {s.form.sku === "" ? "—" : s.form.sku}
          </span>
          <span
            className={`rounded-full px-2 py-0.5 text-[11px] font-black ${
              s.form.isActive
                ? "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-200"
                : "bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-200"
            }`}
          >
            {s.form.isActive ? t(locale, "activeStatus") : t(locale, "inactiveWarning")}
          </span>
        </div>
        <p className="text-[11px] font-semibold text-slate-500">
          {t(locale, "skuImmutableNotice")}
        </p>

        {s.successMessage !== null ? (
          <p className="rounded-lg bg-emerald-50 px-3 py-3 text-center text-base font-black text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
            {t(locale, "updateSuccess")}
          </p>
        ) : (
          <>
            <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
              <div className="flex flex-col gap-2">
                <label className="flex flex-col gap-1">
                  <span className="text-xs font-bold text-slate-500">
                    {t(locale, "productNameAr")}
                  </span>
                  <input
                    value={s.form.nameAr}
                    onChange={(event) => {
                      s.setFormField("nameAr", event.target.value);
                    }}
                    dir="rtl"
                    autoComplete="off"
                    disabled={s.isSubmitting}
                    className={inputClass}
                  />
                </label>
                <label className="flex flex-col gap-1">
                  <span className="text-xs font-bold text-slate-500">
                    {t(locale, "productNameFr")}
                  </span>
                  <input
                    value={s.form.nameFr}
                    onChange={(event) => {
                      s.setFormField("nameFr", event.target.value);
                    }}
                    dir="ltr"
                    autoComplete="off"
                    disabled={s.isSubmitting}
                    className={inputClass}
                  />
                </label>
                <label className="flex flex-col gap-1">
                  <span className="text-xs font-bold text-slate-500">
                    {t(locale, "unitName")}
                  </span>
                  <input
                    value={s.form.baseUnitName}
                    onChange={(event) => {
                      s.setFormField("baseUnitName", event.target.value);
                    }}
                    autoComplete="off"
                    disabled={s.isSubmitting}
                    className={inputClass}
                  />
                </label>
              </div>
              <div className="flex flex-col gap-2">
                <label className="flex flex-col gap-1">
                  <span className="text-xs font-bold text-slate-500">
                    {t(locale, "sellingPrice")}
                  </span>
                  <input
                    value={s.form.sellingPriceDzd}
                    onChange={(event) => {
                      s.setFormField("sellingPriceDzd", event.target.value);
                    }}
                    inputMode="decimal"
                    autoComplete="off"
                    dir="ltr"
                    disabled={s.isSubmitting}
                    className={`${inputClass} font-mono text-base font-black`}
                  />
                </label>
                <label className="flex flex-col gap-1">
                  <span className="text-xs font-bold text-slate-500">
                    {t(locale, "costPrice")}
                  </span>
                  <input
                    value={s.form.costPriceDzd}
                    onChange={(event) => {
                      s.setFormField("costPriceDzd", event.target.value);
                    }}
                    inputMode="decimal"
                    autoComplete="off"
                    dir="ltr"
                    disabled={s.isSubmitting}
                    className={`${inputClass} font-mono text-base font-black`}
                  />
                </label>
                <label className="flex flex-col gap-1">
                  <span className="text-xs font-bold text-slate-500">{t(locale, "barcode")}</span>
                  <input
                    value={s.form.barcode}
                    onChange={(event) => {
                      s.setFormField("barcode", event.target.value);
                    }}
                    autoComplete="off"
                    dir="ltr"
                    disabled={s.isSubmitting}
                    className={`${inputClass} font-mono`}
                  />
                </label>
              </div>
            </div>

            <label className="flex cursor-pointer items-center gap-2 text-sm font-bold">
              <input
                type="checkbox"
                checked={s.form.isActive}
                disabled={s.isSubmitting}
                onChange={(event) => {
                  s.setFormField("isActive", event.target.checked);
                }}
                className="h-4 w-4 accent-emerald-600"
              />
              {t(locale, "activeStatus")}
            </label>
            {!s.form.isActive && (
              <p className="rounded-md bg-amber-50 px-3 py-2 text-xs font-bold text-amber-800 dark:bg-amber-900/30 dark:text-amber-200">
                {t(locale, "inactiveWarning")}
              </p>
            )}

            <div className="flex items-center justify-between rounded-lg bg-slate-100 px-3 py-2 text-sm dark:bg-slate-800">
              <span className="font-bold text-slate-500">{t(locale, "marginLabel")}</span>
              <span className="font-mono font-black">
                {validation.ok
                  ? formatMargin(validation.sellingCentimes, validation.costCentimes, locale)
                  : "…"}
              </span>
            </div>

            {!validation.ok && (
              <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
                {t(locale, validation.key)}
              </p>
            )}
            {s.errorMessage !== null && (
              <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
                {translateBackendError(s.errorMessage, locale)}
              </p>
            )}
          </>
        )}

        <div className="flex gap-2">
          <button
            type="button"
            onClick={close}
            disabled={s.isSubmitting}
            className="h-11 flex-1 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:text-slate-100"
          >
            {t(locale, s.successMessage !== null ? "common.close" : "cancelEdit")}
          </button>
          {s.successMessage === null && (
            <button
              type="button"
              onClick={() => {
                void s.submitUpdate();
              }}
              disabled={!canSubmit}
              className="h-11 flex-[2] rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
            >
              {s.isSubmitting ? (
                <span className="flex items-center justify-center gap-2">
                  <span
                    className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-slate-900 border-t-transparent"
                    aria-hidden="true"
                  />
                  {t(locale, "savingChanges")}
                </span>
              ) : (
                t(locale, "saveChanges")
              )}
            </button>
          )}
        </div>
      </div>
    </ModalShell>
  );
}
