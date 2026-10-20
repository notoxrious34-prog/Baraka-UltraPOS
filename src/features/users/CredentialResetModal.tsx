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
    "w-full rounded border border-line-default bg-surface-raised px-2 py-1.5 text-sm text-ink-primary placeholder:text-ink-muted focus:border-line-strong focus:ring-1 focus:ring-line-strong";
  // ModalShell title is a plain string (shared shell, out of refactor scope):
  // isolate the LTR username with U+2066 LRI … U+2069 PDI instead of <bdi>.
  const title = `${t(locale, "users.resetTitle")} ${String.fromCharCode(0x2066)}${target.username}${String.fromCharCode(0x2069)}`;
  return (
    <ModalShell title={title} onClose={onClose}>
      <div className="flex flex-col gap-3">
        <p className="text-sm text-ink-secondary">
          {t(locale, "users.resetTarget")}{" "}
          <bdi dir="ltr" className="font-mono font-bold text-ink-primary">
            {target.username}
          </bdi>
        </p>
        <p className="text-sm text-ink-muted">{t(locale, "users.resetHint")}</p>
        <label className="flex flex-col gap-1 text-sm text-ink-primary">
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
        <label className="flex flex-col gap-1 text-sm text-ink-primary">
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
          <p className="rounded border border-danger-fg/30 bg-danger-bg px-2 py-1.5 text-sm font-bold text-danger-fg">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={onClose}
            disabled={busy}
            className="rounded border border-line-default bg-surface-sunken px-3 py-1.5 text-sm font-bold text-ink-primary hover:bg-surface-raised disabled:opacity-50"
          >
            {t(locale, "users.formCancel")}
          </button>
          <button
            type="button"
            onClick={() => {
              void handleSave();
            }}
            disabled={!canSave}
            className="rounded bg-ink-primary px-3 py-1.5 text-sm font-bold text-ink-inverse disabled:opacity-50"
          >
            {busy ? t(locale, "users.formSaving") : t(locale, "users.formSave")}
          </button>
        </div>
      </div>
    </ModalShell>
  );
}
