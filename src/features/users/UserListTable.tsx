/**
 * Staff directory table (DSN-030, tokenized DSN-071): dense rows, sticky
 * header, per-row role/status/PIN badges. Pure view — actions lift through
 * callbacks. The sole active Admin is proactively shielded: its row carries a
 * protective badge and its Edit entry point (role/active patch surface) is
 * disabled with an explanatory title (backend LAST_ADMIN_PROTECTED stays the
 * authority; credential reset remains available).
 */

import type { ReactElement } from "react";
import { t } from "@/lib/i18n";
import type { Locale } from "@/types/pos";
import type { UserInfo } from "@/lib/ipc";
import { roleBadgeClass, roleLabel, statusBadgeClass } from "./user-helpers";

interface UserListTableProps {
  locale: Locale;
  users: readonly UserInfo[];
  onEdit: (user: UserInfo) => void;
  onResetCredentials: (user: UserInfo) => void;
}

export function UserListTable({
  locale,
  users,
  onEdit,
  onResetCredentials,
}: UserListTableProps): ReactElement {
  const activeAdminCount = users.filter((user) => user.role === "ADMIN" && user.is_active).length;

  if (users.length === 0) {
    return (
      <div className="flex min-h-0 flex-1 items-center justify-center p-8">
        <p className="max-w-md text-center text-sm text-ink-muted">
          {t(locale, "users.empty")}
        </p>
      </div>
    );
  }
  return (
    <div className="min-h-0 flex-1 overflow-auto">
      <table className="w-full border-collapse text-sm">
        <thead className="sticky top-0 bg-surface-sunken">
          <tr>
            <th className="px-3 py-2 text-start text-xs font-bold text-ink-secondary">
              {t(locale, "users.colUsername")}
            </th>
            <th className="px-3 py-2 text-start text-xs font-bold text-ink-secondary">
              {t(locale, "users.colRole")}
            </th>
            <th className="px-3 py-2 text-start text-xs font-bold text-ink-secondary">
              {t(locale, "users.colStatus")}
            </th>
            <th className="px-3 py-2 text-start text-xs font-bold text-ink-secondary">
              {t(locale, "users.colPin")}
            </th>
            <th className="px-3 py-2 text-start text-xs font-bold text-ink-secondary">
              {t(locale, "users.colCreated")}
            </th>
            <th className="px-3 py-2 text-start text-xs font-bold text-ink-secondary">
              {t(locale, "users.colActions")}
            </th>
          </tr>
        </thead>
        <tbody>
          {users.map((user) => {
            const isLastActiveAdmin =
              user.role === "ADMIN" && user.is_active && activeAdminCount <= 1;
            return (
              <tr key={user.id} className="border-t border-line-default hover:bg-surface-sunken">
                <td className="px-3 py-2 font-mono font-bold text-ink-primary">
                  <bdi dir="ltr">{user.username}</bdi>
                </td>
                <td className="px-3 py-2">
                  <span
                    className={`inline-block rounded-full px-2 py-0.5 text-xs ${roleBadgeClass(user.role)}`}
                  >
                    {roleLabel(user.role, locale)}
                  </span>
                  {isLastActiveAdmin && (
                    <span className="ms-1 inline-block rounded-full border border-warning-fg/20 bg-warning-bg px-2 py-0.5 text-xs font-bold text-warning-fg">
                      {t(locale, "users.lastAdminBadge")}
                    </span>
                  )}
                </td>
                <td className="px-3 py-2">
                  <span
                    className={`inline-block rounded-full px-2 py-0.5 text-xs ${statusBadgeClass(user.is_active)}`}
                  >
                    {user.is_active ? t(locale, "users.active") : t(locale, "users.statusInactive")}
                  </span>
                </td>
                <td className="px-3 py-2">
                  <span className="inline-block rounded border border-line-default bg-surface-sunken px-2 py-0.5 text-xs font-semibold text-ink-secondary">
                    {user.has_pin ? t(locale, "users.pinSet") : t(locale, "users.pinUnset")}
                  </span>
                </td>
                <td className="whitespace-nowrap px-3 py-2 text-xs tabular-nums text-ink-muted">
                  <bdi dir="ltr">{user.created_at}</bdi>
                </td>
                <td className="whitespace-nowrap px-3 py-2">
                  <button
                    type="button"
                    onClick={() => {
                      onEdit(user);
                    }}
                    disabled={isLastActiveAdmin}
                    title={isLastActiveAdmin ? t(locale, "users.lastAdminTitle") : undefined}
                    className="me-2 rounded border border-line-default bg-surface-sunken px-2 py-1 text-xs font-bold text-ink-primary hover:bg-surface-raised disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    {t(locale, "users.edit")}
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      onResetCredentials(user);
                    }}
                    className="rounded border border-warning-fg/20 bg-warning-bg px-2 py-1 text-xs font-bold text-warning-fg hover:opacity-90"
                  >
                    {t(locale, "users.credentials")}
                  </button>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
