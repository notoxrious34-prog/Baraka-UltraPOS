/**
 * Quick-products slot customization dialog (TASK-DSN-076).
 *
 * Modal overlay (standard shell: `fixed inset-0`, no other fixed UI).
 * Eight slot cards (Alt+1..Alt+8) with assign/change/clear; the assign flow
 * queries strictly active products (`includeInactive=false`) and only
 * active products can occupy a slot (store invariant, surfaced inline).
 */

import { useEffect, useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import {
  QUICK_SLOT_COUNT,
  useQuickProductsStore,
} from "@/stores/quick-products-store";
import { ipcSearchCatalog } from "@/lib/ipc";
import { t } from "@/lib/i18n";
import { formatMoney } from "@/lib/money";
import type { CatalogProduct, Locale } from "@/types/pos";

const ASSIGN_SEARCH_LIMIT = 20;
const ASSIGN_DEBOUNCE_MS = 200;

function slotPrice(product: CatalogProduct): string {
  const priced =
    product.units.find((unit) => unit.is_default_selling) ?? product.units[0];
  return priced !== undefined ? priced.sale_price_centimes : "0";
}

function slotLabel(product: CatalogProduct, locale: Locale): string {
  return locale === "ar-DZ" ? product.name_ar : product.name_fr;
}

export function QuickProductsConfigModal(): ReactElement | null {
  const locale = usePosUiStore((state) => state.locale);
  const isOpen = useQuickProductsStore((state) => state.isConfigOpen);
  const slots = useQuickProductsStore((state) => state.slots);
  const [pendingSlot, setPendingSlot] = useState<number | null>(null);
  const [query, setQuery] = useState("");
  const [candidates, setCandidates] = useState<CatalogProduct[]>([]);
  const [searching, setSearching] = useState(false);
  const [warning, setWarning] = useState<string | null>(null);

  // Active-only candidate search while a slot awaits assignment.
  useEffect(() => {
    if (!isOpen || pendingSlot === null) {
      return;
    }
    const timer = window.setTimeout(() => {
      const token = useSessionStore.getState().getSessionToken();
      if (token === null) {
        setCandidates([]);
        return;
      }
      setSearching(true);
      void ipcSearchCatalog(token, query, ASSIGN_SEARCH_LIMIT, false)
        .then((rows) => {
          setCandidates(rows.filter((row) => row.is_active === true));
        })
        .catch(() => {
          setCandidates([]);
        })
        .finally(() => {
          setSearching(false);
        });
    }, ASSIGN_DEBOUNCE_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [isOpen, pendingSlot, query]);

  if (!isOpen) {
    return null;
  }

  const close = (): void => {
    useQuickProductsStore.getState().setConfigOpen(false);
    setPendingSlot(null);
    setQuery("");
    setCandidates([]);
    setWarning(null);
  };

  const assign = (product: CatalogProduct): void => {
    if (pendingSlot === null) {
      return;
    }
    if (product.is_active !== true) {
      setWarning(t(locale, "pos.inactiveProductWarning"));
      return;
    }
    useQuickProductsStore.getState().assignSlot(pendingSlot, product);
    setPendingSlot(null);
    setQuery("");
    setCandidates([]);
    setWarning(null);
  };

  const resetAll = (): void => {
    const token = useSessionStore.getState().getSessionToken();
    if (token === null) {
      return;
    }
    void ipcSearchCatalog(token, "", QUICK_SLOT_COUNT, false)
      .then((rows) => {
        useQuickProductsStore.getState().resetDefaults(rows);
      })
      .catch(() => {
        // Best effort: keep current slots when the seed fetch fails.
      });
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      role="dialog"
      aria-modal="true"
      aria-label={t(locale, "pos.quickModalTitle")}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          close();
        }
      }}
    >
      <div className="flex max-h-[90vh] w-full max-w-2xl flex-col gap-3 overflow-hidden rounded-xl border border-line-default bg-surface-raised p-4 text-ink-primary">
        <div className="flex shrink-0 items-center justify-between gap-2">
          <h2 className="text-base font-black">{t(locale, "pos.quickModalTitle")}</h2>
          <button
            type="button"
            onClick={close}
            aria-label={t(locale, "pos.quickClose")}
            className="rounded-md border border-line-default bg-surface-sunken px-2 py-1 text-xs font-bold text-ink-secondary hover:bg-surface-canvas"
          >
            ✕
          </button>
        </div>

        <div className="grid min-h-0 flex-1 grid-cols-1 gap-2 overflow-y-auto sm:grid-cols-2">
          {slots.map((product, index) => (
            <div
              key={index}
              className="flex min-w-0 flex-col gap-1.5 rounded-lg border border-line-default bg-surface-canvas p-2.5"
            >
              <span className="font-mono text-[11px] font-bold text-ink-muted">
                Alt+{index + 1}
              </span>
              {product === null ? (
                <span className="text-sm font-bold text-ink-muted">
                  {t(locale, "pos.slotEmpty")}
                </span>
              ) : (
                <>
                  <span className="truncate text-sm font-extrabold">
                    {slotLabel(product, locale)}
                  </span>
                  <span className="flex items-center gap-2 font-mono text-[11px] text-ink-muted">
                    <span>{product.sku}</span>
                    <span>·</span>
                    <bdi dir="ltr">{formatMoney(slotPrice(product), locale)}</bdi>
                  </span>
                </>
              )}
              <span className="mt-auto flex items-center gap-1.5 pt-1">
                <button
                  type="button"
                  onClick={() => {
                    setPendingSlot(index);
                    setWarning(null);
                  }}
                  className={`rounded-md px-2 py-1 text-xs font-bold outline-none focus-visible:ring-2 focus-visible:ring-primary-600/50 ${
                    pendingSlot === index
                      ? "bg-amber-500 text-white"
                      : "bg-slate-200 text-slate-700 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-200"
                  }`}
                >
                  {product === null
                    ? t(locale, "pos.assignProduct")
                    : t(locale, "pos.changeProduct")}
                </button>
                {product !== null && (
                  <button
                    type="button"
                    onClick={() => {
                      useQuickProductsStore.getState().clearSlot(index);
                    }}
                    aria-label={t(locale, "pos.clearSlot")}
                    className="rounded-md border border-line-default px-2 py-1 text-xs font-bold text-ink-secondary hover:bg-surface-sunken"
                  >
                    {t(locale, "pos.clearSlot")}
                  </button>
                )}
              </span>
            </div>
          ))}
        </div>

        {pendingSlot !== null && (
          <div className="flex min-h-0 shrink-0 flex-col gap-2 rounded-lg border border-amber-500/40 bg-amber-500/5 p-2.5">
            <input
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
              }}
              placeholder={t(locale, "pos.quickSearchHint")}
              autoComplete="off"
              aria-label={t(locale, "pos.quickSearchHint")}
              className="h-10 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm font-semibold outline-none focus:border-primary-600 focus:ring-2 focus:ring-primary-600/30 dark:border-slate-700 dark:bg-slate-900"
            />
            <p className="text-[11px] font-semibold text-ink-muted">
              {t(locale, "pos.inactiveProductWarning")}
            </p>
            {searching && (
              <p className="text-[11px] font-semibold text-ink-muted">…</p>
            )}
            {!searching && candidates.length > 0 && (
              <ul className="flex max-h-40 flex-col gap-1 overflow-y-auto">
                {candidates.map((candidate) => (
                  <li key={candidate.product_id}>
                    <button
                      type="button"
                      onClick={() => {
                        assign(candidate);
                      }}
                      className="flex w-full items-center gap-2 rounded-md border border-line-default bg-surface-raised px-2 py-1.5 text-start text-xs font-bold hover:border-primary-600"
                    >
                      <span className="min-w-0 flex-1 truncate">
                        {slotLabel(candidate, locale)}
                      </span>
                      <span className="shrink-0 font-mono text-ink-muted">
                        {candidate.sku}
                      </span>
                      <bdi dir="ltr" className="shrink-0 font-mono">
                        {formatMoney(slotPrice(candidate), locale)}
                      </bdi>
                    </button>
                  </li>
                ))}
              </ul>
            )}
            {warning !== null && (
              <p className="text-xs font-bold text-red-600 dark:text-red-300">
                {warning}
              </p>
            )}
          </div>
        )}

        <div className="flex shrink-0 items-center justify-end gap-2">
          <button
            type="button"
            onClick={resetAll}
            className="rounded-lg border border-line-default px-3 py-1.5 text-xs font-bold text-ink-secondary hover:bg-surface-sunken"
          >
            {t(locale, "pos.resetDefaults")}
          </button>
          <button
            type="button"
            onClick={close}
            className="rounded-lg bg-primary-600 px-3 py-1.5 text-xs font-bold text-white hover:bg-primary-700"
          >
            {t(locale, "pos.quickClose")}
          </button>
        </div>
      </div>
    </div>
  );
}
