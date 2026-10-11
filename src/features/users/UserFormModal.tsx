/**
 * Create/edit operator modal (DSN-030). Create requires username + password
 * (+ optional PIN, masked per ADR-079); edit patches role and active flag
 * against a read-only username. Backend codes surface via `userErrorMessage`.
 */

import { useState, type ReactElement } from "react";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import { t } from "@/lib/i18n";
import type { Locale } from "@/types/pos";
import type { UserInfo } from "@/lib/ipc";
import {
  isRoleCode,
  roleLabel,
  roleOptions,
  userErrorMessage,
  validatePassword,
  validatePin,
  validateUsername,
} from "./user-helpers";

interface UserFormModalProps {
  locale: Locale;
  sessionToken: string;
  /** `null` = create mode; a user = edit mode. */
  target: UserInfo | null;
  onClose: () => void;
  onSaved: () => void;
  createUser: (
    token: string,
    payload: { username: string; password: string; pin?: string; role: string },
  ) => Promise<UserInfo>;
  updateUser: (
    token: string,
    payload: { user_id: string; role?: string; is_active?: boolean },
  ) => Promise<UserInfo>;
}

export function UserFormModal({
  locale,
  sessionToken,
  target,
  onClose,
  onSaved,
  createUser,
  updateUser,
}: UserFormModalProps): ReactElement {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [pin, setPin] = useState("");
  const [role, setRole] = useState<string>(target?.role ?? "VENDEUR");
  const [isActive, setIsActive] = useState(target?.is_active ?? true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const usernameOk = target !== null || validateUsername(username.trim());
  const passwordOk = target !== null || validatePassword(password);
  const pinOk = pin === "" || validatePin(pin);
  const canSave = !busy && usernameOk && passwordOk && pinOk;

  async function handleSave(): Promise<void> {
    if (!canSave) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      if (target === null) {
        await createUser(sessionToken, {
          username: username.trim(),
          password,
          ...(pin === "" ? {} : { pin }),
          role,
        });
      } else {
        await updateUser(sessionToken, {
          user_id: target.id,
          ...(isRoleCode(role) && role !== target.role ? { role } : {}),
          ...(isActive !== target.is_active ? { is_active: isActive } : {}),
        });
      }
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
    <ModalShell
      title={t(locale, target === null ? "users.formCreate" : "users.formEdit")}
      onClose={onClose}
    >
      <div className="flex flex-col gap-3">
        {target === null ? (
          <label className="flex flex-col gap-1 text-sm">
            <span className="font-bold">{t(locale, "users.formUsername")}</span>
            <input
              className={inputClass}
              value={username}
              disabled={busy}
              onChange={(event) => {
                setUsername(event.target.value);
              }}
              placeholder="caisse1"
            />
          </label>
        ) : (
          <p className="font-mono text-sm font-bold">
            <bdi dir="ltr">{target.username}</bdi>
          </p>
        )}
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-bold">{t(locale, "users.formRole")}</span>
          <select
            className={inputClass}
            value={target === null ? role : (isRoleCode(role) ? role : target.role)}
            disabled={busy}
            onChange={(event) => {
              setRole(event.target.value);
            }}
          >
            {roleOptions().map((code) => (
              <option key={code} value={code}>
                {roleLabel(code, locale)}
              </option>
            ))}
          </select>
        </label>
        {target === null ? (
          <>
            <label className="flex flex-col gap-1 text-sm">
              <span className="font-bold">{t(locale, "users.formPassword")}</span>
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
              <span className="font-bold">{t(locale, "users.formPin")}</span>
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
          </>
        ) : (
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={isActive}
              disabled={busy}
              onChange={(event) => {
                setIsActive(event.target.checked);
              }}
            />
            <span className="font-bold">{t(locale, "users.formActive")}</span>
          </label>
        )}
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
