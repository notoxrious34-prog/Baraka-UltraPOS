/**
 * Data Exchange dropdown (TASK-DSN-048): single entry point for all
 * catalog import/export operations. Replaces the three scattered header
 * buttons (import / starter-pack / export) plus the template download.
 *
 * Pure presentational + local open state; the container owns IPC.
 * `defaultOpen` exists only for the node-only static-markup test harness.
 */

import { useEffect, useRef, useState, type ReactElement } from "react";
import { t } from "@/lib/i18n";
import type { Locale } from "@/types/pos";

export interface DataExchangeMenuProps {
  locale: Locale;
  onOpenImportModal: () => void;
  onImportStarterPack: () => void;
  onExportCsv: () => void;
  onDownloadTemplate: () => void;
  /** Test-only: render the panel open on first paint (static markup). */
  defaultOpen?: boolean;
}

export function DataExchangeMenu(props: DataExchangeMenuProps): ReactElement {
  const { locale } = props;
  const [open, setOpen] = useState(props.defaultOpen === true);
  const rootRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    const onPointerDown = (event: MouseEvent): void => {
      if (rootRef.current !== null && !rootRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open ]);

  const fire = (action: () => void): void => {
    setOpen(false);
    action();
  };

  const itemClass =
    "flex w-full items-center gap-2 px-3 py-2 text-start text-sm font-bold text-slate-700 hover:bg-slate-100 dark:text-slate-200 dark:hover:bg-slate-800";

  return (
    <div ref={rootRef} className="relative shrink-0">
      <button
        type="button"
        onClick={() => {
          setOpen((current) => !current);
        }}
        aria-haspopup="menu"
        aria-expanded={open}
        className="flex h-10 items-center gap-2 rounded-lg border border-neutral-300 bg-white px-3 py-2 text-sm font-medium text-neutral-700 shadow-sm hover:bg-neutral-50 dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-200 dark:hover:bg-neutral-700"
      >
        <svg
          aria-hidden="true"
          width="16"
          height="16"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.8"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M8 2v9" />
          <path d="M4.5 7.5 8 11l3.5-3.5" />
          <path d="M2.5 13.5h11" />
        </svg>
        {t(locale, "cat.dataExchange")}
        <span aria-hidden="true" className="text-xs opacity-60">
          {open ? "▲" : "▼"}
        </span>
      </button>
      {open && (
        <div
          role="menu"
          className="absolute end-0 z-30 mt-1 w-80 overflow-hidden rounded-lg border border-slate-200 bg-white py-1 shadow-xl dark:border-slate-700 dark:bg-slate-900"
        >
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              fire(props.onOpenImportModal);
            }}
            className={itemClass}
          >
            <span aria-hidden="true">📥</span>
            {t(locale, "cat.exchangeImport")}
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              fire(props.onImportStarterPack);
            }}
            className={itemClass}
          >
            <span aria-hidden="true">📦</span>
            {t(locale, "cat.exchangePack")}
          </button>
          <div aria-hidden="true" className="mx-3 my-1 border-t border-slate-200 dark:border-slate-700" />
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              fire(props.onExportCsv);
            }}
            className={itemClass}
          >
            <span aria-hidden="true">📤</span>
            {t(locale, "cat.exchangeExport")}
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              fire(props.onDownloadTemplate);
            }}
            className={itemClass}
          >
            <span aria-hidden="true">📄</span>
            {t(locale, "cat.exchangeTemplate")}
          </button>
        </div>
      )}
    </div>
  );
}
