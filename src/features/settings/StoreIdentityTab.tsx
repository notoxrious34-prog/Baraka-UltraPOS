/**
 * Store identity & fiscal mentions tab (DSN-031): loads via
 * `getStoreIdentity()`, persists via `saveStoreIdentity(identity,
 * deviceKind)`. Fiscal numbers and phones are LTR-isolated.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t, type I18nKey } from "@/lib/i18n";
import { useSessionStore } from "@/stores/session-store";
import type { Locale } from "@/types/pos";
import {
  getStoreIdentity,
  saveStoreIdentity,
  type DeviceKind,
  type StoreIdentity,
} from "@/lib/ipc";
import { validateFiscalField, validatePhone, validateStoreName } from "./settings-helpers";

interface StoreIdentityTabProps {
  locale: Locale;
}

const EMPTY: StoreIdentity = {
  storeName: "",
  storeAddress: "",
  storePhone: "",
  nif: "",
  nis: "",
  rc: "",
  ai: "",
  ticketFooterMessage: "",
};

export function StoreIdentityTab({ locale }: StoreIdentityTabProps): ReactElement {
  const hasPermission = useSessionStore((state) => state.hasPermission);
  const canWrite = hasPermission("SYSTEM_SETTINGS_MANAGE");
  const [identity, setIdentity] = useState<StoreIdentity>(EMPTY);
  const [deviceKind, setDeviceKind] = useState<DeviceKind>("physical");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);

  useEffect(() => {
    let cancelled = false;
    getStoreIdentity()
      .then((state) => {
        if (!cancelled) {
          setIdentity(state.identity);
          setDeviceKind(state.deviceKind);
          setLoading(false);
        }
      })
      .catch((failure: unknown) => {
        if (!cancelled) {
          setNotice({ ok: false, text: String(failure) });
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const patch = (field: keyof StoreIdentity, value: string): void => {
    setIdentity((previous) => ({ ...previous, [field]: value }));
  };

  const nameOk = validateStoreName(identity.storeName);
  const phoneOk = validatePhone(identity.storePhone);
  const fiscalOk =
    validateFiscalField(identity.rc) &&
    validateFiscalField(identity.nif) &&
    validateFiscalField(identity.nis) &&
    validateFiscalField(identity.ai);
  const canSave = !loading && !saving && nameOk && phoneOk && fiscalOk && canWrite;

  async function handleSave(): Promise<void> {
    if (!canSave || !canWrite) {
      return;
    }
    setSaving(true);
    setNotice(null);
    try {
      const state = await saveStoreIdentity(identity, deviceKind);
      setIdentity(state.identity);
      setDeviceKind(state.deviceKind);
      setNotice({ ok: true, text: t(locale, "settings.saved") });
    } catch (failure) {
      setNotice({ ok: false, text: String(failure) });
    } finally {
      setSaving(false);
    }
  }

  const inputClass =
    "w-full rounded border border-slate-300 bg-white px-2 py-1.5 text-sm dark:border-slate-600 dark:bg-slate-800";
  const labelClass = "flex flex-col gap-1 text-sm";
  if (loading) {
    return (
      <p className="p-4 text-sm text-slate-500">{t(locale, "settings.loading")}</p>
    );
  }
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
      <label className={labelClass}>
        <span className="font-bold">{t(locale, "settings.storeName")}</span>
        <input
          className={inputClass}
          value={identity.storeName}
          disabled={saving}
          onChange={(event) => {
            patch("storeName", event.target.value);
          }}
        />
      </label>
      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
        <label className={labelClass}>
          <span className="font-bold">{t(locale, "settings.phone")}</span>
          <bdi dir="ltr" className="contents">
            <input
              className={`${inputClass} tabular-nums`}
              value={identity.storePhone}
              disabled={saving}
              inputMode="tel"
              onChange={(event) => {
                patch("storePhone", event.target.value);
              }}
            />
          </bdi>
        </label>
        <label className={labelClass}>
          <span className="font-bold">{t(locale, "settings.address")}</span>
          <input
            className={inputClass}
            value={identity.storeAddress}
            disabled={saving}
            onChange={(event) => {
              patch("storeAddress", event.target.value);
            }}
          />
        </label>
      </div>
      <fieldset className="rounded border border-slate-200 p-3 dark:border-slate-700">
        <legend className="px-1 text-xs font-bold text-slate-500">
          {t(locale, "settings.fiscalTitle")}
        </legend>
        <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
          {(
            [
              ["rc", "settings.rc"],
              ["nif", "settings.nif"],
              ["nis", "settings.nis"],
              ["ai", "settings.ai"],
            ] as Array<[keyof StoreIdentity, I18nKey]>
          ).map(([field, key]) => (
            <label key={field} className={labelClass}>
              <span className="font-bold">{t(locale, key)}</span>
              <bdi dir="ltr" className="contents">
                <input
                  className={`${inputClass} tabular-nums`}
                  value={identity[field]}
                  disabled={saving}
                  onChange={(event) => {
                    patch(field, event.target.value);
                  }}
                />
              </bdi>
            </label>
          ))}
        </div>
      </fieldset>
      <label className={labelClass}>
        <span className="font-bold">{t(locale, "settings.footer")}</span>
        <textarea
          className={inputClass}
          rows={2}
          value={identity.ticketFooterMessage}
          disabled={saving}
          onChange={(event) => {
            patch("ticketFooterMessage", event.target.value);
          }}
        />
      </label>
      <label className="flex items-center gap-2 text-sm">
        <span className="font-bold">{t(locale, "settings.deviceKind")}</span>
        <select
          className="rounded border border-slate-300 bg-white px-2 py-1.5 text-sm dark:border-slate-600 dark:bg-slate-800"
          value={deviceKind}
          disabled={saving}
          onChange={(event) => {
            setDeviceKind(event.target.value === "capture" ? "capture" : "physical");
          }}
        >
          <option value="physical">{t(locale, "settings.devicePhysical")}</option>
          <option value="capture">{t(locale, "settings.deviceCapture")}</option>
        </select>
      </label>
      {notice !== null && (
        <p
          className={`rounded px-2 py-1.5 text-sm font-bold ${
            notice.ok
              ? "bg-green-50 text-green-700 dark:bg-green-900/30 dark:text-green-200"
              : "bg-red-50 text-red-700 dark:bg-red-900/30 dark:text-red-200"
          }`}
        >
          {notice.text}
        </p>
      )}
      <div>
        <button
          type="button"
          onClick={() => {
            void handleSave();
          }}
          disabled={!canSave}
          title={canWrite ? undefined : t(locale, "settings.identityWriteReservedGuidance")}
          className="rounded bg-accent-500 px-3 py-1.5 text-sm font-bold text-slate-950 disabled:opacity-50"
        >
          {saving ? t(locale, "settings.saving") : t(locale, "settings.save")}
        </button>
      </div>
    </div>
  );
}
