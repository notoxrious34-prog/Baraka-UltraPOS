/**
 * Category management modal (TASK-DSN-044): taxonomy CRUD plus one-click
 * Quincaillerie Starter Pack seeding for the Master Catalog Hub.
 *
 * Mounted as an overlay inside `CatalogModule` (ADR-067: `fixed` only for
 * modal overlays). All IPC goes through the real backend contract from
 * TASK-BRK-065 — `category_list` / `category_create` / `category_update`
 * (code immutable) / `category_seed_starter` (resolves the full row list).
 * No floats, no hardcoded colors, logical properties for RTL.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import {
  ipcCreateCategory,
  ipcListCategories,
  ipcSeedStarterCategories,
  ipcUpdateCategory,
} from "@/lib/ipc";
import { usePosUiStore } from "@/stores/pos-ui-store";
import type { Category } from "@/types/pos";

export interface CategoryManageModalProps {
  isOpen: boolean;
  token: string;
  onClose: () => void;
  onChanged: () => void;
}

export interface CategoryManagePanelProps {
  token: string;
  onChanged: () => void;
}

/** Trim + uppercase a category code (`" plomb "` -> `"PLOMB"`). Pure. */
export function normalizeCategoryCode(raw: string): string {
  return raw.trim().toUpperCase();
}

/** Client-side validation for the inline add/edit form. Pure. */
export function validateCategoryForm(data: {
  code: string;
  nameFr: string;
  nameAr: string;
}): Partial<Record<"code" | "nameFr" | "nameAr", "cat.required">> {
  const errors: Partial<Record<"code" | "nameFr" | "nameAr", "cat.required">> = {};
  if (normalizeCategoryCode(data.code) === "") {
    errors.code = "cat.required";
  }
  if (data.nameFr.trim() === "") {
    errors.nameFr = "cat.required";
  }
  if (data.nameAr.trim() === "") {
    errors.nameAr = "cat.required";
  }
  return errors;
}

const inputClass =
  "h-10 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm font-bold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800";

export function CategoryManagePanel({
  token,
  onChanged,
}: CategoryManagePanelProps): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [seeding, setSeeding] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [code, setCode] = useState("");
  const [nameFr, setNameFr] = useState("");
  const [nameAr, setNameAr] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [attempted, setAttempted] = useState(false);

  const reload = async (tok: string, alive: () => boolean): Promise<void> => {
    setLoading(true);
    try {
      const rows = await ipcListCategories(tok, true);
      if (alive()) {
        setCategories(rows);
        setError(null);
      }
    } catch (failure: unknown) {
      if (alive()) {
        setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
      }
    } finally {
      if (alive()) {
        setLoading(false);
      }
    }
  };

  useEffect(() => {
    setCode("");
    setNameFr("");
    setNameAr("");
    setEditingId(null);
    setAttempted(false);
    setBusy(false);
    setSeeding(false);
    setError(null);
    setNotice(null);
    let cancelled = false;
    const alive = (): boolean => !cancelled;
    void reload(token, alive);
    return () => {
      cancelled = true;
    };
  }, [token]);

  const errors = attempted ? validateCategoryForm({ code, nameFr, nameAr }) : {};

  const startEdit = (category: Category): void => {
    setEditingId(category.id);
    setCode(category.code);
    setNameFr(category.name_fr);
    setNameAr(category.name_ar);
    setAttempted(false);
    setError(null);
    setNotice(null);
  };

  const cancelEdit = (): void => {
    setEditingId(null);
    setCode("");
    setNameFr("");
    setNameAr("");
    setAttempted(false);
  };

  const submit = (): void => {
    if (busy) {
      return;
    }
    // The backend keeps codes immutable: edits carry names + flag only.
    const draft =
      editingId === null
        ? { code, nameFr, nameAr }
        : { code: "EDIT", nameFr, nameAr };
    setAttempted(true);
    if (Object.keys(validateCategoryForm(draft)).length > 0) {
      return;
    }
    setBusy(true);
    setError(null);
    setNotice(null);
    const done = (next: Category[]): void => {
      setCategories(next);
      setBusy(false);
      cancelEdit();
      onChanged();
    };
    if (editingId === null) {
      ipcCreateCategory(token, {
        code: normalizeCategoryCode(code),
        name_fr: nameFr.trim(),
        name_ar: nameAr.trim(),
      })
        .then((created) => {
          done([...categories, created]);
        })
        .catch((failure: unknown) => {
          setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
          setBusy(false);
        });
      return;
    }
    const target = categories.find((row) => row.id === editingId);
    if (target === undefined) {
      setError("NOT_FOUND");
      setBusy(false);
      return;
    }
    ipcUpdateCategory(token, {
      category_id: editingId,
      name_fr: nameFr.trim(),
      name_ar: nameAr.trim(),
      is_active: target.is_active,
    })
      .then((updated) => {
        done(categories.map((row) => (row.id === updated.id ? updated : row)));
      })
      .catch((failure: unknown) => {
        setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
        setBusy(false);
      });
  };

  const toggleActive = (category: Category): void => {
    if (busy || seeding) {
      return;
    }
    setBusy(true);
    setError(null);
    ipcUpdateCategory(token, {
      category_id: category.id,
      name_fr: category.name_fr,
      name_ar: category.name_ar,
      is_active: !category.is_active,
    })
      .then((updated) => {
        setCategories((prev) => prev.map((row) => (row.id === updated.id ? updated : row)));
        onChanged();
      })
      .catch((failure: unknown) => {
        setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
      })
      .finally(() => {
        setBusy(false);
      });
  };

  const seedStarter = (): void => {
    if (seeding || busy) {
      return;
    }
    setSeeding(true);
    setError(null);
    setNotice(null);
    ipcSeedStarterCategories(token)
      .then((rows) => {
        setCategories(rows);
        setNotice(`${rows.length}`);
        onChanged();
      })
      .catch((failure: unknown) => {
        setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
      })
      .finally(() => {
        setSeeding(false);
      });
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-900">
        <div className="flex h-14 shrink-0 items-center justify-between bg-primary-900 px-4 text-white">
          <h2 className="text-base font-extrabold">{t(locale, "cat.manageTitle")}</h2>
        </div>

        <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
          {error !== null && (
            <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
              {translateBackendError(error, locale)}
            </p>
          )}
          {notice !== null && (
            <p className="rounded-md bg-green-50 px-3 py-2 text-sm font-bold text-green-700 dark:bg-green-900/30 dark:text-green-200">
              {t(locale, "cat.starterTitle")}
              {" — "}
              {notice}
            </p>
          )}

          <div
            className={`rounded-xl border p-3 ${
              categories.length === 0
                ? "border-accent-500 bg-amber-50 dark:bg-amber-900/20"
                : "border-slate-200 bg-slate-50 dark:border-slate-700 dark:bg-slate-800"
            }`}
          >
            <h3 className="text-sm font-black">
              {"🔧 "}
              {t(locale, "cat.starterTitle")}
            </h3>
            <p className="mt-1 text-xs font-bold opacity-70">{t(locale, "cat.starterCopy")}</p>
            <button
              type="button"
              onClick={seedStarter}
              disabled={seeding || busy}
              className="mt-2 h-10 rounded-lg bg-accent-500 px-4 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-50"
            >
              {seeding ? t(locale, "cat.seeding") : t(locale, "cat.installStarter")}
            </button>
          </div>

          <h3 className="text-xs font-black uppercase opacity-70">
            {editingId === null ? t(locale, "cat.newCategory") : t(locale, "cat.editAction")}
          </h3>
          <div className="grid grid-cols-1 gap-2 md:grid-cols-3">
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.code")}</span>
              <input
                value={code}
                dir="ltr"
                disabled={editingId !== null}
                onChange={(event) => {
                  setCode(event.target.value.toUpperCase());
                }}
                placeholder="PLOMB"
                className={`${inputClass} font-mono disabled:opacity-60`}
              />
              {errors.code !== undefined && (
                <span className="text-xs font-bold text-red-600 dark:text-red-300">
                  {t(locale, errors.code)}
                </span>
              )}
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.nameFr")}</span>
              <input
                value={nameFr}
                onChange={(event) => {
                  setNameFr(event.target.value);
                }}
                className={inputClass}
              />
              {errors.nameFr !== undefined && (
                <span className="text-xs font-bold text-red-600 dark:text-red-300">
                  {t(locale, errors.nameFr)}
                </span>
              )}
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.nameArOptional")}</span>
              <input
                value={nameAr}
                dir="auto"
                onChange={(event) => {
                  setNameAr(event.target.value);
                }}
                className={inputClass}
              />
              {errors.nameAr !== undefined && (
                <span className="text-xs font-bold text-red-600 dark:text-red-300">
                  {t(locale, errors.nameAr)}
                </span>
              )}
            </label>
          </div>
          <div className="flex gap-2">
            {editingId !== null && (
              <button
                type="button"
                onClick={cancelEdit}
                disabled={busy}
                className="h-10 rounded-lg bg-slate-200 px-4 text-sm font-bold hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-800 dark:hover:bg-slate-700"
              >
                {t(locale, "cat.cancel")}
              </button>
            )}
            <button
              type="button"
              onClick={submit}
              disabled={busy}
              className="h-10 flex-1 rounded-lg bg-primary-600 text-sm font-black text-white hover:bg-primary-900 disabled:opacity-50"
            >
              {busy ? t(locale, "cat.saving") : t(locale, "cat.save")}
            </button>
          </div>

          {loading ? (
            <p className="text-xs font-bold">{t(locale, "cash.loading")}</p>
          ) : (
            <div className="overflow-x-auto rounded-xl border border-slate-200 dark:border-slate-700">
              <table className="w-full border-collapse text-xs">
                <thead className="bg-slate-100 dark:bg-slate-800">
                  <tr>
                    <th scope="col" className="px-3 py-2 text-start font-black">
                      {t(locale, "cat.code")}
                    </th>
                    <th scope="col" className="px-3 py-2 text-start font-black">
                      {t(locale, "cat.nameFr")}
                    </th>
                    <th scope="col" className="px-3 py-2 text-start font-black">
                      {t(locale, "cat.nameAr")}
                    </th>
                    <th scope="col" className="px-3 py-2 text-start font-black">
                      {t(locale, "cat.colStatus")}
                    </th>
                    <th scope="col" className="px-3 py-2 text-start font-black">
                      {t(locale, "cat.colActions")}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {categories.map((category) => (
                    <tr
                      key={category.id}
                      className="border-t border-slate-100 dark:border-slate-800"
                    >
                      <td className="px-3 py-1.5 font-mono font-black">{category.code}</td>
                      <td className="px-3 py-1.5 font-bold">{category.name_fr}</td>
                      <td className="px-3 py-1.5 font-bold">{category.name_ar}</td>
                      <td className="px-3 py-1.5">
                        <button
                          type="button"
                          disabled={busy || seeding}
                          onClick={() => {
                            toggleActive(category);
                          }}
                          aria-label={`${category.code} ${t(locale, "cat.productActiveLabel")}`}
                          className={
                            category.is_active
                              ? "rounded-md bg-green-100 px-2 py-0.5 font-black text-green-700 hover:bg-green-200 disabled:opacity-50 dark:bg-green-900/40 dark:text-green-200"
                              : "rounded-md bg-slate-200 px-2 py-0.5 font-black text-slate-500 hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:text-slate-300"
                          }
                        >
                          {category.is_active ? t(locale, "cat.active") : t(locale, "cat.inactive")}
                        </button>
                      </td>
                      <td className="px-3 py-1.5">
                        <button
                          type="button"
                          disabled={busy || seeding}
                          onClick={() => {
                            startEdit(category);
                          }}
                          aria-label={`${t(locale, "cat.editAction")} ${category.code}`}
                          className="rounded-md bg-slate-200 px-2 py-1 text-xs font-black hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:hover:bg-slate-600"
                        >
                          ✏️
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {categories.length === 0 && (
                <p className="px-4 py-6 text-center text-sm font-bold opacity-60">
                  {t(locale, "cat.noProducts")}
                </p>
              )}
            </div>
          )}
        </div>
    </div>
  );
}

/**
 * Category management modal: thin overlay wrapper around the shared
 * `CategoryManagePanel` (Escape/backdrop dismissal).
 * Retained for compatibility; new surfaces mount the panel directly.
 */
export function CategoryManageModal({
  isOpen,
  token,
  onClose,
  onChanged,
}: CategoryManageModalProps): ReactElement | null {
  const locale = usePosUiStore((state) => state.locale);
  if (!isOpen) {
    return null;
  }
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      role="dialog"
      aria-modal="true"
      aria-label={t(locale, "cat.manageTitle")}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          onClose();
        }
      }}
    >
      <div className="flex max-h-[90vh] w-full max-w-2xl flex-col overflow-hidden rounded-xl border border-slate-200 bg-white shadow-2xl dark:border-slate-700 dark:bg-slate-900">
        <CategoryManagePanel token={token} onChanged={onChanged} />
      </div>
    </div>
  );
}
