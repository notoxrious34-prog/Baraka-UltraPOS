/**
 * Store identity + print diagnostics modal (TASK-BRK-056).
 *
 * "Identité & Impression" section: 8-field identity form in ticket
 * print order, diagnostics kind selector, AR|FR test language, and the
 * synchronous test-print flow (layout preview + capture name + device
 * badge). The whole section renders nothing without the Admin role
 * (SystemSettingsManage lives only in the Admin grant — Rust remains
 * authoritative). Physical serial inputs are NOT duplicated here; the
 * hint references the printer serial settings (printer.serial_*).
 */

import { useEffect, type ReactElement } from "react";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import { TestPrintPreview } from "@/components/pos/modals/TestPrintPreview";
import { t, translateBackendError, type I18nKey } from "@/lib/i18n";
import type { StoreIdentity } from "@/lib/ipc";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import {
  canManageIdentity,
  deviceBadge,
  isIdentityDirty,
  maxFor,
  useStoreIdentityStore,
} from "@/stores/store-identity-store";

const FIELDS: Array<{ field: keyof StoreIdentity; multiline: boolean }> = [
  { field: "storeName", multiline: false },
  { field: "storeAddress", multiline: false },
  { field: "storePhone", multiline: false },
  { field: "nif", multiline: false },
  { field: "nis", multiline: false },
  { field: "rc", multiline: false },
  { field: "ai", multiline: false },
  { field: "ticketFooterMessage", multiline: true },
];

const LABEL_KEYS: Record<keyof StoreIdentity, I18nKey> = {
  storeName: "settings.identity.storeName",
  storeAddress: "settings.identity.storeAddress",
  storePhone: "settings.identity.storePhone",
  nif: "settings.identity.nif",
  nis: "settings.identity.nis",
  rc: "settings.identity.rc",
  ai: "settings.identity.ai",
  ticketFooterMessage: "settings.identity.footer",
};

const PLACEHOLDER_KEYS: Record<keyof StoreIdentity, I18nKey> = {
  storeName: "settings.identity.storeNamePh",
  storeAddress: "settings.identity.storeAddressPh",
  storePhone: "settings.identity.storePhonePh",
  nif: "settings.identity.nifPh",
  nis: "settings.identity.nisPh",
  rc: "settings.identity.rcPh",
  ai: "settings.identity.aiPh",
  ticketFooterMessage: "settings.identity.footerPh",
};

export function StoreIdentityModal(): ReactElement | null {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const session = useSessionStore((state) => state.session);
  const store = useStoreIdentityStore();

  useEffect(() => {
    if (session?.role === "ADMIN") {
      void store.load(locale).catch(() => undefined);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session?.role]);

  if (!canManageIdentity()) {
    return null;
  }

  const dirty = isIdentityDirty(store);
  const invalidCount = Object.keys(store.fieldErrors).length;
  const saveDisabled = store.saving || invalidCount > 0;
  const badge = store.result === null ? null : deviceBadge(locale, store.result.deviceOutcome);

  const withMax = (template: string, max: number): string =>
    template.replace("{max}", String(max));

  return (
    <ModalShell title={t(locale, "settings.identity.title")} onClose={closeModal}>
      <div className="flex flex-col gap-4">
        {store.savedFlash && (
          <p role="status" className="rounded-md bg-emerald-50 px-3 py-2 text-sm font-bold text-emerald-800">
            ✓ {t(locale, "settings.identity.saved")}
          </p>
        )}
        {store.saveError !== null && (
          <p role="alert" className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-800">
            ✗ {translateBackendError(store.saveError, locale)}
          </p>
        )}
        {FIELDS.map(({ field, multiline }) => {
          const messageKey = store.fieldErrors[field];
          const inputId = `identity-${field}`;
          const shared =
            "min-h-[44px] w-full rounded-md border px-3 py-2 text-sm focus-visible:outline-2 focus-visible:outline-offset-2";
          return (
            <label key={field} htmlFor={inputId} className="flex flex-col gap-1 text-sm font-bold">
              {t(locale, LABEL_KEYS[field])}
              {field === "storeName" && <span aria-hidden="true"> *</span>}
              {multiline ? (
                <textarea
                  id={inputId}
                  rows={3}
                  value={store.form[field]}
                  placeholder={t(locale, PLACEHOLDER_KEYS[field])}
                  onChange={(event) => store.setField(field, event.target.value)}
                  onBlur={() => store.blurField(field)}
                  className={shared}
                />
              ) : (
                <input
                  id={inputId}
                  type="text"
                  value={store.form[field]}
                  placeholder={t(locale, PLACEHOLDER_KEYS[field])}
                  onChange={(event) => store.setField(field, event.target.value)}
                  onBlur={() => store.blurField(field)}
                  className={shared}
                />
              )}
              {field === "ticketFooterMessage" && (
                <span className="text-xs font-normal opacity-70">
                  {t(locale, "settings.identity.footerHint")}
                </span>
              )}
              {messageKey !== undefined && (
                <span role="alert" className="text-xs font-bold text-red-700">
                  {withMax(t(locale, messageKey), maxFor(field))}
                </span>
              )}
            </label>
          );
        })}
        <button
          type="button"
          disabled={saveDisabled}
          onClick={() => {
            void store.save().catch(() => undefined);
          }}
          className="min-h-[44px] rounded-md bg-primary-900 px-4 py-2 text-sm font-black text-white disabled:opacity-40"
        >
          {store.saving ? t(locale, "settings.identity.saving") : t(locale, "settings.identity.save")}
        </button>

        <fieldset className="flex flex-col gap-2 rounded-md border p-3">
          <legend className="px-1 text-sm font-black">{t(locale, "settings.identity.kind")}</legend>
          {(["capture", "physical"] as const).map((kind) => (
            <label key={kind} className="flex min-h-[44px] items-center gap-2 text-sm">
              <input
                type="radio"
                name="identity-kind"
                checked={store.deviceKind === kind}
                onChange={() => store.setDeviceKind(kind)}
              />
              {t(
                locale,
                kind === "capture" ? "settings.identity.kindCapture" : "settings.identity.kindPhysical",
              )}
            </label>
          ))}
          {store.deviceKind === "physical" && (
            <p className="text-xs opacity-80">{t(locale, "settings.identity.physicalHint")}</p>
          )}
        </fieldset>

        <div className="flex flex-wrap items-center gap-2">
          <span className="text-sm font-black">{t(locale, "settings.identity.language")}</span>
          {(["ar", "fr"] as const).map((language) => (
            <button
              key={language}
              type="button"
              disabled={store.testing}
              onClick={() => store.setLanguage(language)}
              aria-pressed={store.language === language}
              className="min-h-[44px] rounded-md border px-4 py-2 text-sm font-black disabled:opacity-40"
            >
              {language.toUpperCase()}
            </button>
          ))}
          <button
            type="button"
            disabled={store.testing}
            onClick={() => {
              void store.runTest().catch(() => undefined);
            }}
            className="min-h-[44px] rounded-md bg-slate-800 px-4 py-2 text-sm font-black text-white disabled:opacity-40"
          >
            {store.testing ? t(locale, "settings.identity.testing") : t(locale, "settings.identity.test")}
          </button>
        </div>
        {dirty && (
          <p className="text-xs opacity-80">{t(locale, "settings.identity.unsavedHint")}</p>
        )}

        <div aria-live="polite" className="flex flex-col gap-2">
          {store.testError !== null && (
            <p role="alert" className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-800">
              ✗ {translateBackendError(store.testError, locale)}
            </p>
          )}
          {badge !== null && store.result !== null && (
            <>
              <p
                role="status"
                className={`rounded-md px-3 py-2 text-sm font-black ${
                  badge.ok ? "bg-emerald-50 text-emerald-900" : "bg-red-50 text-red-900"
                }`}
              >
                {badge.icon} {badge.label}
              </p>
              <p className="text-sm font-bold">{t(locale, "settings.identity.previewCaption")}</p>
              <TestPrintPreview lines={store.result.preview} />
              <p className="text-xs">
                {t(locale, "settings.identity.captureCaption")}:{" "}
                <span className="font-mono">{store.result.captureFileName}</span>
              </p>
              <p className="text-xs opacity-70">{t(locale, "settings.identity.captureHint")}</p>
              <p className="text-xs">
                {t(locale, "settings.identity.bands")}: {store.result.decodeSummary.bands} ·{" "}
                {t(locale, "settings.identity.feedLines")}: {store.result.decodeSummary.feedLines} ·{" "}
                {t(locale, "settings.identity.unknownSpans")}: {store.result.decodeSummary.unknownSpans}
              </p>
              {store.result.decodeSummary.unknownSpans > 0 && (
                <p role="alert" className="text-xs font-bold text-amber-800">
                  ⚠ {t(locale, "settings.identity.unknownWarning")}
                </p>
              )}
            </>
          )}
        </div>

        <p className="text-xs opacity-60">{t(locale, "settings.identity.drillRef")}</p>
      </div>
    </ModalShell>
  );
}
