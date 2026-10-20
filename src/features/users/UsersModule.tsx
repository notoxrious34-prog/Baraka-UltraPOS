/**
 * Staff directory + access control (DSN-030, tokenized DSN-071). Loads
 * operators via `ipcListUsers`, shows HUD counts, and opens the create/edit
 * and credential reset modals. Mounted strictly behind the
 * `hasPermission("USER_MANAGE")` gate in `App.tsx`.
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

  const activeAdminTotal = users.filter((user) => user.role === "ADMIN" && user.is_active).length;

  return (
    <section className="flex h-full min-h-0 flex-col overflow-hidden bg-surface-canvas">
      <header className="flex shrink-0 flex-wrap items-center gap-3 p-4 pb-2">
        <div className="me-auto">
          <h1 className="text-lg font-extrabold text-ink-primary">{t(locale, "users.title")}</h1>
          <p className="text-xs text-ink-muted">{t(locale, "users.subtitle")}</p>
        </div>
        <button
          type="button"
          onClick={() => {
            setDialog({ kind: "form", target: null });
          }}
          className="rounded bg-ink-primary px-3 py-1.5 text-sm font-bold text-ink-inverse"
        >
          {t(locale, "users.newUser")}
        </button>
      </header>
      <div className="grid shrink-0 grid-cols-2 gap-2 px-4 pb-2 lg:grid-cols-4">
        {cards.map((card) => (
          <div
            key={card.label}
            className="rounded-md border border-line-default bg-surface-raised px-3 py-2"
          >
            <p className="text-xs text-ink-muted">{card.label}</p>
            <p className="text-xl font-extrabold tabular-nums text-ink-primary">{card.value}</p>
          </div>
        ))}
      </div>
      {loading ? (
        <div className="flex min-h-0 flex-1 items-center justify-center">
          <p className="text-sm text-ink-muted">{t(locale, "users.loading")}</p>
        </div>
      ) : loadError !== null ? (
        <div className="flex min-h-0 flex-1 items-center justify-center p-8">
          <p className="rounded border border-danger-fg/30 bg-danger-bg px-4 py-3 text-sm font-bold text-danger-fg">
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
          isProtected={
            dialog.target !== null &&
            dialog.target.role === "ADMIN" &&
            dialog.target.is_active &&
            activeAdminTotal <= 1
          }
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
