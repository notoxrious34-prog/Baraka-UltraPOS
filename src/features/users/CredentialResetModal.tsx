/**
 * Credential reset modal (DSN-030): optional replacement password and/or PIN
 * (masked per ADR-079), at least one required, via `ipcResetUserCredentials`.
 */

import { useState, type ReactElement } from "react";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import { t } from "@/lib/i18n";
import type { Locale } from "@/types/pos";
import type { UserInfo } from "@/lib/ipc";
import { userErrorMessage, validatePassword, validatePin } from "./user-helpers";

interface CredentialResetModalProps {
  locale: Locale;
  sessionToken: string;
  target: UserInfo;
  onClose: () => void;
  onSaved: () => void;
  resetCredentials: (
    token: string,
    payload: { user_id: string; password?: string; pin?: string },
  ) => Promise<void>;
}

export function CredentialResetModal({
  locale,
  sessionToken,
  target,
  onClose,
  onSaved,
  resetCredentials,
}: CredentialResetModalProps): ReactElement {
  const [password, setPassword] = useState("");
  const [pin, setPin] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const passwordOk = password === "" || validatePassword(password);
  const pinOk = pin === "" || validatePin(pin);
  const hasOne = password !== "" || pin !== "";
  const canSave = !busy && passwordOk && pinOk && hasOne;

  async function handleSave(): Promise<void> {
    if (!canSave) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await resetCredentials(sessionToken, {
        user_id: target.id,
        ...(password === "" ? {} : { password }),
        ...(pin === "" ? {} : { pin }),
      });
      onSaved();
    } catch (failure) {
      setError(userErrorMessage(String(failure), locale));
    } finally {
      setBusy(false);
    }
  }

  const inputClass =
    "w-full rounded border border-slate-300 bg-white px-2 py-1.5 text-sm dark:border-slate-600 dark:bg-slate-800";
  return (
    <ModalShell title={`${t(locale, "users.resetTitle")} ${target.username}`} onClose={onClose}>
      <div className="flex flex-col gap-3">
        <p className="text-sm text-slate-500 dark:text-slate-400">{t(locale, "users.resetHint")}</p>
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-bold">{t(locale, "users.resetPassword")}</span>
          <input
            type="password"
            className={inputClass}
            value={password}
            disabled={busy}
            autoComplete="new-password"
            onChange={(event) => {
              setPassword(event.target.value);
            }}
          />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-bold">{t(locale, "users.resetPin")}</span>
          <input
            type="password"
            inputMode="numeric"
            className={inputClass}
            value={pin}
            disabled={busy}
            autoComplete="new-password"
            onChange={(event) => {
              setPin(event.target.value);
            }}
          />
        </label>
        {error !== null && (
          <p className="rounded bg-red-50 px-2 py-1.5 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={onClose}
            disabled={busy}
            className="rounded bg-slate-200 px-3 py-1.5 text-sm font-bold hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600"
          >
            {t(locale, "users.formCancel")}
          </button>
          <button
            type="button"
            onClick={() => {
              void handleSave();
            }}
            disabled={!canSave}
            className="rounded bg-accent-500 px-3 py-1.5 text-sm font-bold text-slate-950 disabled:opacity-50"
          >
            {busy ? t(locale, "users.formSaving") : t(locale, "users.formSave")}
          </button>
        </div>
      </div>
    </ModalShell>
  );
}
