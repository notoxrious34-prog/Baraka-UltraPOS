/**
 * Quick-products palette (TASK-DSN-069, extended TASK-DSN-076).
 *
 * Up to 8 single-click / Alt+1..8 chips for barcode-less bulk items, mounted
 * inside the discovery column (CartPanel structurally unreachable). Bounded
 * height, no layout shift: `shrink-0`, two compact rows max, inner scroll only
 * on overflow. Empty slots render dashed; the last dispatched chip flashes.
 * When the palette is disabled the component renders nothing (the discovery
 * grid reclaims the vertical space); the gear button opens the slot
 * customization dialog.
 */

import { useEffect, useRef, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import {
  QUICK_SLOT_COUNT,
  useQuickProductsStore,
} from "@/stores/quick-products-store";
import { ipcSearchCatalog } from "@/lib/ipc";
import { t } from "@/lib/i18n";

const FLASH_MS = 600;

export function QuickProductPalette(): ReactElement | null {
  const locale = usePosUiStore((state) => state.locale);
  const enabled = useQuickProductsStore((state) => state.enabled);
  const slots = useQuickProductsStore((state) => state.slots);
  const hydrated = useQuickProductsStore((state) => state.hydrated);
  const lastFiredSlot = useQuickProductsStore((state) => state.lastFiredSlot);
  const flashTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // First-run seeding: stored config wins; otherwise the first catalog rows.
  useEffect(() => {
    if (hydrated) {
      return;
    }
    const token = useSessionStore.getState().getSessionToken();
    if (token === null) {
      useQuickProductsStore.getState().hydrate([]);
      return;
    }
    let cancelled = false;
    void ipcSearchCatalog(token, "", QUICK_SLOT_COUNT)
      .then((rows) => {
        if (!cancelled) {
          useQuickProductsStore.getState().hydrate(rows);
        }
      })
      .catch(() => {
        if (!cancelled) {
          useQuickProductsStore.getState().hydrate([]);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [hydrated]);

  useEffect(() => {
    if (lastFiredSlot === null) {
      return;
    }
    if (flashTimer.current !== null) {
      clearTimeout(flashTimer.current);
    }
    flashTimer.current = setTimeout(() => {
      useQuickProductsStore.getState().clearFired();
    }, FLASH_MS);
    return () => {
      if (flashTimer.current !== null) {
        clearTimeout(flashTimer.current);
        flashTimer.current = null;
      }
    };
  }, [lastFiredSlot]);

  // Disabled palette renders nothing: the discovery grid below reclaims the
  // vertical space (no layout shift, no reserved height).
  if (!enabled) {
    return null;
  }

  return (
    <div
      role="toolbar"
      aria-label="Quick products"
      className="flex max-h-16 shrink-0 flex-wrap gap-1 overflow-y-auto"
    >
      {slots.map((product, index) => {
        const label =
          product === null
            ? "…"
            : locale === "ar-DZ"
              ? product.name_ar
              : product.name_fr;
        const fired = lastFiredSlot === index;
        return (
          <button
            key={index}
            type="button"
            disabled={product === null}
            onClick={() => {
              useQuickProductsStore.getState().dispatchSlot(index);
            }}
            title={label}
            aria-label={`Alt+${index + 1} · ${label}`}
            className={`flex h-7 min-w-0 items-center gap-1 rounded-md border px-1.5 text-[11px] font-bold outline-none transition-colors focus-visible:ring-2 focus-visible:ring-primary-600/50 disabled:cursor-default ${
              product === null
                ? "border-dashed border-slate-300 text-slate-400 dark:border-slate-700 dark:text-slate-600"
                : fired
                  ? "border-emerald-600 bg-emerald-600 text-white"
                  : "border-slate-300 bg-white text-slate-700 hover:border-primary-600 hover:bg-sky-50 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-sky-950/40"
            }`}
          >
            <kbd className="rounded border border-line-default bg-surface-sunken px-1 py-0.5 font-mono text-[10px]">
              Alt+{index + 1}
            </kbd>
            <span className="max-w-24 truncate">{label}</span>
          </button>
        );
      })}
      <button
        key="quick-config"
        type="button"
        onClick={() => {
          useQuickProductsStore.getState().setConfigOpen(true);
        }}
        title={t(locale, "pos.customizeQuickProducts")}
        aria-label={t(locale, "pos.customizeQuickProducts")}
        className="flex h-7 shrink-0 items-center gap-1 rounded-md border border-slate-300 bg-white px-1.5 text-[11px] font-bold text-slate-700 outline-none transition-colors hover:border-primary-600 hover:bg-sky-50 focus-visible:ring-2 focus-visible:ring-primary-600/50 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-sky-950/40"
      >
        <span aria-hidden="true">⚙️</span>
      </button>
    </div>
  );
}
