/**
 * Operations menu (TASK-DSN-023, ADR-067): topbar dropdown consolidating
 * the six former floating action buttons. Role gating mirrors the old
 * buttons exactly — documents for every authenticated role, stock for
 * GERANT/ADMIN, system for ADMIN only.
 */

import { useEffect, useRef, useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { useConversionModalStore } from "@/stores/conversion-modal-store";
import { useReturnModalStore } from "@/stores/return-modal-store";
import { useWarehouseTransferStore } from "@/stores/warehouse-transfer-store";
import { useStockAdjustModalStore } from "@/stores/stock-adjust-modal-store";
import { useBackupModalStore } from "@/stores/backup-modal-store";
import { t } from "@/lib/i18n";
import type { ActiveModal } from "@/types/pos";

export type OperationId =
  | "conversion"
  | "return"
  | "warehouse-transfer"
  | "stock-adjust"
  | "backup"
  | "identity";

export type OperationSectionKey = "documents" | "stock" | "system";

export interface OperationSection {
  readonly key: OperationSectionKey;
  readonly items: readonly OperationId[];
}

/**
 * Pure role-gating map (unit-tested): which sections and items a session
 * role may see. Unknown/empty roles see the document section only when
 * authenticated — the component itself renders nothing without a session.
 */
export function operationsForRole(role: string): readonly OperationSection[] {
  const manager = role === "GERANT" || role === "ADMIN";
  const admin = role === "ADMIN";
  const sections: OperationSection[] = [
    { key: "documents", items: ["conversion", "return"] },
  ];
  if (manager) {
    sections.push({
      key: "stock",
      items: ["warehouse-transfer", "stock-adjust"],
    });
  }
  if (admin) {
    sections.push({ key: "system", items: ["backup", "identity"] });
  }
  return sections;
}

/** Payload-store openers injected for unit tests; production wires the real stores. */
export interface OperationDeps {
  readonly openPosModal: (modal: Exclude<ActiveModal, "none">) => void;
  readonly openConversion: () => void;
  readonly openReturn: () => void;
  readonly openWarehouseTransfer: () => void;
  readonly openStockAdjust: () => void;
  readonly openBackup: () => void;
}

/**
 * Pure action dispatcher (unit-tested): maps an operation id to its
 * payload-store opener plus the pos-ui modal key. The caller closes the
 * dropdown after dispatch.
 */
export function runOperation(id: OperationId, deps: OperationDeps): void {
  switch (id) {
    case "conversion":
      deps.openConversion();
      deps.openPosModal("conversion");
      break;
    case "return":
      deps.openReturn();
      deps.openPosModal("return");
      break;
    case "warehouse-transfer":
      deps.openWarehouseTransfer();
      deps.openPosModal("warehouse-transfer");
      break;
    case "stock-adjust":
      deps.openStockAdjust();
      deps.openPosModal("stock-adjust");
      break;
    case "backup":
      deps.openBackup();
      deps.openPosModal("backup");
      break;
    case "identity":
      deps.openPosModal("identity");
      break;
  }
}

const ITEM_ICON: Record<OperationId, string> = {
  conversion: "⇄",
  return: "↩",
  "warehouse-transfer": "⇄",
  "stock-adjust": "⚖",
  backup: "🛡",
  identity: "🖨",
};

const SECTION_TITLE_KEY: Record<OperationSectionKey, "topbar.operationsDocuments" | "topbar.operationsStock" | "topbar.operationsSystem"> = {
  documents: "topbar.operationsDocuments",
  stock: "topbar.operationsStock",
  system: "topbar.operationsSystem",
};

export function OperationsMenu({
  defaultOpen = false,
}: {
  /** Test/seeding hook only — production always mounts closed. */
  readonly defaultOpen?: boolean;
}): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const openModal = usePosUiStore((state) => state.openModal);
  const session = useSessionStore((state) => state.session);

  const handleSelect = (id: OperationId): void => {
    runOperation(id, {
      openPosModal: openModal,
      openConversion: () => {
        useConversionModalStore.getState().openModal("devis_to_bl");
      },
      openReturn: () => {
        useReturnModalStore.getState().openModal();
      },
      openWarehouseTransfer: () => {
        void useWarehouseTransferStore.getState().openModal();
      },
      openStockAdjust: () => {
        useStockAdjustModalStore.getState().openModal();
      },
      openBackup: () => {
        void useBackupModalStore.getState().openModal();
      },
    });
  };

  if (session === null) {
    return <></>;
  }
  return (
    <OperationsMenuView
      role={session.role}
      locale={locale}
      defaultOpen={defaultOpen}
      onSelect={handleSelect}
    />
  );
}

/**
 * Presentational dropdown (unit-tested via static markup with explicit
 * props — the repo harness is node-only, so store-connected components
 * always render their initial snapshot). Owns the open state and closes
 * itself after every selection, outside click, or Escape.
 */
export function OperationsMenuView({
  role,
  locale,
  defaultOpen = false,
  onSelect,
}: {
  readonly role: string;
  readonly locale: "fr-DZ" | "ar-DZ";
  readonly defaultOpen?: boolean;
  readonly onSelect: (id: OperationId) => void;
}): ReactElement {
  const [isOpen, setIsOpen] = useState(defaultOpen);
  const containerRef = useRef<HTMLDivElement | null>(null);

  // Click-outside + Escape dismiss the popover.
  useEffect(() => {
    if (!isOpen) {
      return;
    }
    const onPointerDown = (event: PointerEvent): void => {
      if (
        containerRef.current !== null &&
        !containerRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        setIsOpen(false);
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [isOpen]);

  const sections = operationsForRole(role);

  const itemLabel = (id: OperationId): string => {
    switch (id) {
      case "conversion":
        return t(locale, "conversion.open");
      case "return":
        return t(locale, "return.returnModalTitle");
      case "warehouse-transfer":
        return t(locale, "warehousesTitle");
      case "stock-adjust":
        return t(locale, "stockAdjust.title");
      case "backup":
        return t(locale, "backups.title");
      case "identity":
        return t(locale, "settings.identity.title");
    }
  };

  const handleSelect = (id: OperationId): void => {
    onSelect(id);
    setIsOpen(false);
  };

  return (
    <div ref={containerRef} className="relative">
      <button
        type="button"
        aria-expanded={isOpen}
        aria-haspopup="true"
        onClick={() => {
          setIsOpen((value) => !value);
        }}
        className="flex h-9 items-center gap-1.5 rounded-lg bg-white/10 px-3 text-xs font-bold text-white hover:bg-white/20 focus:outline-none focus:ring-2 focus:ring-accent-400"
      >
        <svg
          className="h-4 w-4"
          viewBox="0 0 16 16"
          fill="currentColor"
          aria-hidden="true"
        >
          <rect x="1" y="1" width="6" height="6" rx="1.5" />
          <rect x="9" y="1" width="6" height="6" rx="1.5" />
          <rect x="1" y="9" width="6" height="6" rx="1.5" />
          <rect x="9" y="9" width="6" height="6" rx="1.5" />
        </svg>
        <span>{t(locale, "topbar.operations")}</span>
        <span aria-hidden="true">▾</span>
      </button>
      {isOpen && (
        <div className="absolute end-0 top-full z-50 mt-1.5 w-64 rounded-xl border border-slate-200 bg-white p-1.5 text-slate-900 shadow-2xl dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100">
          {sections.map((section, sectionIndex) => (
            <div key={section.key}>
              {sectionIndex > 0 && (
                <div className="mx-2 my-1 border-t border-slate-200 dark:border-slate-700" />
              )}
              <p className="px-2.5 pb-1 pt-1.5 text-[11px] font-black uppercase tracking-wide text-slate-500 dark:text-slate-400">
                {t(locale, SECTION_TITLE_KEY[section.key])}
              </p>
              {section.items.map((id) => (
                <button
                  key={id}
                  type="button"
                  onClick={() => {
                    handleSelect(id);
                  }}
                  className="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-start text-xs font-bold hover:bg-slate-100 disabled:opacity-40 dark:hover:bg-slate-700/60"
                >
                  <span aria-hidden="true" className="w-5 text-center">
                    {ITEM_ICON[id]}
                  </span>
                  <span>{itemLabel(id)}</span>
                </button>
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
