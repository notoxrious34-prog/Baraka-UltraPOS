/**
 * Supplier create/edit form modal: name mandatory (mirrored client-side,
 * backend enforces), all other fields optional. Bilingual errors only.
 */

import { useState, type ReactElement } from "react";
import { t, translateBackendError, type I18nKey } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSupplierStore } from "@/stores/supplier-store";
import type { SupplierView } from "@/types/supplier";

interface Props {
  token: string;
  initial: SupplierView | null;
  onClose: () => void;
  onSaved: (view: SupplierView) => void;
}

function blankToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed === "" ? null : trimmed;
}

export function SupplierFormModal({ token, initial, onClose, onSaved }: Props): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const create = useSupplierStore((state) => state.create);
  const update = useSupplierStore((state) => state.update);
  const [name, setName] = useState(initial?.name ?? "");
  const [phone, setPhone] = useState(initial?.phone ?? "");
  const [address, setAddress] = useState(initial?.address ?? "");
  const [nif, setNif] = useState(initial?.nif ?? "");
  const [rc, setRc] = useState(initial?.rc ?? "");
  const [notes, setNotes] = useState(initial?.notes ?? "");
  const [isActive, setIsActive] = useState(initial?.isActive ?? true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (): Promise<void> => {
    if (busy) {
      return;
    }
    if (name.trim() === "") {
      setError("SUPPLIER_NAME_REQUIRED");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const view =
        initial === null
          ? await create(token, {
              name: name.trim(),
              phone: blankToNull(phone),
              address: blankToNull(address),
              nif: blankToNull(nif),
              rc: blankToNull(rc),
              notes: blankToNull(notes),
            })
          : await update(token, initial.id, {
              name: name.trim(),
              phone: blankToNull(phone),
              address: blankToNull(address),
              nif: blankToNull(nif),
              rc: blankToNull(rc),
              notes: blankToNull(notes),
              isActive,
            });
      onSaved(view);
    } catch (failure: unknown) {
      setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setBusy(false);
    }
  };

  const field = (
    key: I18nKey,
    value: string,
    setValue: (value: string) => void,
  ): ReactElement => (
    <label className="flex flex-col gap-1">
      <span className="text-xs font-bold text-slate-500">{t(locale, key)}</span>
      <input
        value={value}
        onChange={(event) => {
          setValue(event.target.value);
        }}
        className="h-10 rounded-lg border border-slate-300 bg-white px-3 text-sm font-bold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800"
      />
    </label>
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 p-4">
      <div className="flex max-h-full w-full max-w-lg flex-col gap-3 overflow-y-auto rounded-xl bg-white p-4 dark:bg-slate-900">
        <h3 className="text-base font-black">{initial === null ? t(locale, "sup.new") : t(locale, "sup.edit")}</h3>
        {field("sup.name", name, setName)}
        {field("sup.phone", phone, setPhone)}
        {field("sup.address", address, setAddress)}
        {field("sup.nif", nif, setNif)}
        {field("sup.rc", rc, setRc)}
        {field("sup.notes", notes, setNotes)}
        {initial !== null && (
          <label className="flex items-center gap-2 text-xs font-bold">
            <input
              type="checkbox"
              checked={isActive}
              onChange={(event) => {
                setIsActive(event.target.checked);
              }}
            />
            {t(locale, "sup.active")}
          </label>
        )}
        {error !== null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {translateBackendError(error, locale)}
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
            disabled={busy || name.trim() === ""}
            className="h-11 flex-[2] rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
          >
            {busy ? t(locale, "sup.saving") : t(locale, "sup.save")}
          </button>
        </div>
      </div>
    </div>
  );
}
