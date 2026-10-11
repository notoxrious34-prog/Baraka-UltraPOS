/**
 * Staff directory table (DSN-030): dense rows, sticky header, per-row
 * role/status/PIN badges. Pure view — actions lift through callbacks.
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
  if (users.length === 0) {
    return (
      <div className="flex min-h-0 flex-1 items-center justify-center p-8">
        <p className="max-w-md text-center text-sm text-slate-500 dark:text-slate-400">
          {t(locale, "users.empty")}
        </p>
      </div>
    );
  }
  return (
    <div className="min-h-0 flex-1 overflow-auto">
      <table className="w-full border-collapse text-sm">
        <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
          <tr>
            <th className="px-3 py-2 text-start text-xs font-bold text-slate-600 dark:text-slate-300">
              {t(locale, "users.colUsername")}
            </th>
            <th className="px-3 py-2 text-start text-xs font-bold text-slate-600 dark:text-slate-300">
              {t(locale, "users.colRole")}
            </th>
            <th className="px-3 py-2 text-start text-xs font-bold text-slate-600 dark:text-slate-300">
              {t(locale, "users.colStatus")}
            </th>
            <th className="px-3 py-2 text-start text-xs font-bold text-slate-600 dark:text-slate-300">
              {t(locale, "users.colPin")}
            </th>
            <th className="px-3 py-2 text-start text-xs font-bold text-slate-600 dark:text-slate-300">
              {t(locale, "users.colCreated")}
            </th>
            <th className="px-3 py-2 text-start text-xs font-bold text-slate-600 dark:text-slate-300">
              {t(locale, "users.colActions")}
            </th>
          </tr>
        </thead>
        <tbody>
          {users.map((user) => (
            <tr
              key={user.id}
              className="border-t border-slate-200 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800/50"
            >
              <td className="px-3 py-2 font-mono font-bold text-slate-900 dark:text-slate-100">
                <bdi dir="ltr">{user.username}</bdi>
              </td>
              <td className="px-3 py-2">
                <span
                  className={`inline-block rounded-full px-2 py-0.5 text-xs font-bold ${roleBadgeClass(user.role)}`}
                >
                  {roleLabel(user.role, locale)}
                </span>
              </td>
              <td className="px-3 py-2">
                <span
                  className={`inline-block rounded-full px-2 py-0.5 text-xs font-bold ${statusBadgeClass(user.is_active)}`}
                >
                  {user.is_active ? t(locale, "users.active") : t(locale, "users.statusInactive")}
                </span>
              </td>
              <td className="px-3 py-2">
                <span className="inline-block rounded bg-slate-100 px-2 py-0.5 text-xs font-semibold text-slate-600 dark:bg-slate-700 dark:text-slate-300">
                  {user.has_pin ? t(locale, "users.pinSet") : t(locale, "users.pinUnset")}
                </span>
              </td>
              <td className="whitespace-nowrap px-3 py-2 text-xs tabular-nums text-slate-500 dark:text-slate-400">
                <bdi dir="ltr">{user.created_at}</bdi>
              </td>
              <td className="whitespace-nowrap px-3 py-2">
                <button
                  type="button"
                  onClick={() => {
                    onEdit(user);
                  }}
                  className="mr-2 rounded bg-slate-200 px-2 py-1 text-xs font-bold text-slate-800 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-100 dark:hover:bg-slate-600"
                >
                  {t(locale, "users.edit")}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    onResetCredentials(user);
                  }}
                  className="rounded bg-amber-200 px-2 py-1 text-xs font-bold text-amber-900 hover:bg-amber-300 dark:bg-amber-900/50 dark:text-amber-100 dark:hover:bg-amber-900"
                >
                  {t(locale, "users.credentials")}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
