/**
 * Admin-only staff directory + access control (DSN-030). Loads operators via
 * `ipcListUsers`, shows HUD counts, and opens the create/edit and credential
 * reset modals. Mounted strictly behind the ADMIN gate in `App.tsx`.
 */

import { useCallback, useEffect, useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { t } from "@/lib/i18n";
import {
  ipcCreateUser,
  ipcListUsers,
  ipcResetUserCredentials,
  ipcUpdateUser,
  type UserInfo,
} from "@/lib/ipc";
import { UserListTable } from "./UserListTable";
import { UserFormModal } from "./UserFormModal";
import { CredentialResetModal } from "./CredentialResetModal";
import { computeUserMetrics, userErrorMessage } from "./user-helpers";

type Dialog = { kind: "form"; target: UserInfo | null } | { kind: "reset"; target: UserInfo };

export function UsersModule(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [dialog, setDialog] = useState<Dialog | null>(null);

  const refresh = useCallback(async () => {
    if (session === null) {
      return;
    }
    setLoading(true);
    setLoadError(null);
    try {
      setUsers(await ipcListUsers(session.token));
    } catch (failure) {
      setLoadError(userErrorMessage(String(failure), locale));
    } finally {
      setLoading(false);
    }
  }, [session, locale]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const metrics = computeUserMetrics(users);
  const cards: Array<{ label: string; value: number }> = [
    { label: t(locale, "users.total"), value: metrics.total },
    { label: t(locale, "users.admins"), value: metrics.admins },
    { label: t(locale, "users.cashiers"), value: metrics.cashAndSales },
    { label: t(locale, "users.inactive"), value: metrics.inactive },
  ];

  return (
    <section className="flex h-full min-h-0 flex-col overflow-hidden bg-slate-50 dark:bg-slate-900">
      <header className="flex shrink-0 flex-wrap items-center gap-3 p-4 pb-2">
        <div className="mr-auto">
          <h1 className="text-lg font-extrabold">{t(locale, "users.title")}</h1>
          <p className="text-xs text-slate-500 dark:text-slate-400">
            {t(locale, "users.subtitle")}
          </p>
        </div>
        <button
          type="button"
          onClick={() => {
            setDialog({ kind: "form", target: null });
          }}
          className="rounded bg-accent-500 px-3 py-1.5 text-sm font-bold text-slate-950"
        >
          {t(locale, "users.newUser")}
        </button>
      </header>
      <div className="grid shrink-0 grid-cols-2 gap-2 px-4 pb-2 lg:grid-cols-4">
        {cards.map((card) => (
          <div
            key={card.label}
            className="rounded-md bg-white px-3 py-2 shadow-sm dark:bg-slate-800"
          >
            <p className="text-xs text-slate-500 dark:text-slate-400">{card.label}</p>
            <p className="text-xl font-extrabold tabular-nums">{card.value}</p>
          </div>
        ))}
      </div>
      {loading ? (
        <div className="flex min-h-0 flex-1 items-center justify-center">
          <p className="text-sm text-slate-500">{t(locale, "users.loading")}</p>
        </div>
      ) : loadError !== null ? (
        <div className="flex min-h-0 flex-1 items-center justify-center p-8">
          <p className="rounded bg-red-50 px-4 py-3 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {loadError}
          </p>
        </div>
      ) : (
        <UserListTable
          locale={locale}
          users={users}
          onEdit={(user) => {
            setDialog({ kind: "form", target: user });
          }}
          onResetCredentials={(user) => {
            setDialog({ kind: "reset", target: user });
          }}
        />
      )}
      {session !== null && dialog?.kind === "form" && (
        <UserFormModal
          locale={locale}
          sessionToken={session.token}
          target={dialog.target}
          onClose={() => {
            setDialog(null);
          }}
          onSaved={() => {
            setDialog(null);
            void refresh();
          }}
          createUser={ipcCreateUser}
          updateUser={ipcUpdateUser}
        />
      )}
      {session !== null && dialog?.kind === "reset" && (
        <CredentialResetModal
          locale={locale}
          sessionToken={session.token}
          target={dialog.target}
          onClose={() => {
            setDialog(null);
          }}
          onSaved={() => {
            setDialog(null);
            void refresh();
          }}
          resetCredentials={ipcResetUserCredentials}
        />
      )}
    </section>
  );
}
