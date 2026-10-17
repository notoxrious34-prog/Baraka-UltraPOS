/**
 * Sovereign keyboard contract (Architecture Section 15).
 *
 * Global keydown listener. Inside text inputs only Escape, F1–F12 and
 * search-input Enter are intercepted; everything else keeps native behavior.
 */

import { useEffect } from "react";
import { useCartStore } from "@/stores/cart-store";
import { useCashSessionStore } from "@/stores/cash-session-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { ipcCreateDraft } from "@/lib/ipc";
import { t, translateBackendError } from "@/lib/i18n";
import { creditUsageBp, formatMoney } from "@/lib/money";
import type { DocType } from "@/types/pos";

const SEARCH_INPUT_ID = "pos-search-input";

function focusSearch(): void {
  const element = document.getElementById(SEARCH_INPUT_ID);
  if (element instanceof HTMLInputElement) {
    element.focus();
    element.select();
  }
}

function isTextInput(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  );
}

/** True for a bare printable character keystroke (no modifier). */
export function isPrintableKey(event: {
  key: string;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
}): boolean {
  return event.key.length === 1 && !event.ctrlKey && !event.altKey && !event.metaKey;
}

/** True when the active customer blocks checkout on credit grounds. */
export function isCreditBlocked(): boolean {
  const { customer } = useCartStore.getState();
  if (customer.kind !== "named") {
    return false;
  }
  if (customer.policy === "DISABLED") {
    return false;
  }
  if (customer.policy === "UNLIMITED") {
    return false;
  }
  if (customer.limitCentimes === null || customer.balanceCentimes === null) {
    return false;
  }
  try {
    return creditUsageBp(customer.balanceCentimes, customer.limitCentimes) > 10_000n;
  } catch {
    return false;
  }
}

function cycleDocType(current: DocType): DocType {
  if (current === "TICKET") {
    return "FACTURE";
  }
  if (current === "FACTURE") {
    return "BL";
  }
  return "TICKET";
}

async function parkSale(): Promise<void> {
  const cart = useCartStore.getState();
  const session = useSessionStore.getState();
  const ui = usePosUiStore.getState();
  if (cart.lines.length === 0 || session.session === null || session.warehouse === null) {
    return;
  }
  try {
    await ipcCreateDraft(
      session.session.token,
      session.warehouse.id,
      cart.customer,
      cart.lines.map((line) => {
        const unit = line.units[line.unitIndex];
        return {
          product_id: line.productId,
          unit_id: unit !== undefined ? unit.unitId : "",
          quantity_micro: line.quantityMicro,
          unit_price_centimes: null,
          line_discount_centimes: line.lineDiscountCentimes,
        };
      }),
      null,
      ui.locale === "ar-DZ" ? "AR" : "FR",
    );
    cart.clearCart();
    ui.setBanner(t(ui.locale, "parked.ok"));
    focusSearch();
  } catch (error: unknown) {
    const code = typeof error === "string" ? error : "INTERNAL_ERROR";
    ui.setBanner(translateBackendError(code, ui.locale));
  }
}

/** Mount once at the shell root. No reactive subscriptions inside. */
export function usePosKeyboard(): void {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      const ui = usePosUiStore.getState();
      const cart = useCartStore.getState();
      const inInput = isTextInput(event.target);
      const modalOpen = ui.activeModal !== "none";

      if (event.key === "Escape") {
        if (modalOpen) {
          ui.closeModal();
          event.preventDefault();
        } else if (inInput && event.target instanceof HTMLElement) {
          event.target.blur();
        } else {
          usePosUiStore.getState().setSelectedLineIndex(0);
        }
        return;
      }

      // Function keys are always intercepted (never typed into inputs).
      if (event.key === "F1") {
        event.preventDefault();
        if (!modalOpen) {
          focusSearch();
        }
        return;
      }
      if (event.key === "F2") {
        event.preventDefault();
        ui.openModal("customer");
        return;
      }
      if (event.key === "F3") {
        event.preventDefault();
        if (!modalOpen) {
          const line = cart.lines[ui.selectedLineIndex];
          if (line !== undefined) {
            const result = cart.cycleLineUnit(line.key);
            if (result.cycled) {
              const name = ui.locale === "ar-DZ" ? result.unitNameAr : result.unitNameFr;
              ui.setBanner(
                `${t(ui.locale, "pos.unitChanged")} ${name} (${formatMoney(result.salePriceCentimes, ui.locale)})`,
              );
            } else {
              ui.setBanner(t(ui.locale, "pos.singleUnitNotice"));
            }
          }
        }
        return;
      }
      if (event.key === "F4") {
        event.preventDefault();
        if (!modalOpen && cart.lines.length > 0) {
          ui.openModal("quantity");
        }
        return;
      }
      if (event.key === "F5") {
        event.preventDefault();
        if (!modalOpen && cart.lines.length > 0) {
          ui.openModal("discount");
        }
        return;
      }
      if (event.key === "F6") {
        event.preventDefault();
        if (!modalOpen) {
          ui.openModal("parked");
        }
        return;
      }
      if (event.key === "F8") {
        event.preventDefault();
        if (!modalOpen) {
          void parkSale();
        }
        return;
      }
      if (event.key === "F10") {
        event.preventDefault();
        if (!modalOpen) {
          cart.setDocType(cycleDocType(cart.docType));
        }
        return;
      }
      // TASK-DSN-049: F11 has no cockpit action, but the WebView2 default
      // (fullscreen toggle) must never fire while POS is active.
      if (event.key === "F11") {
        event.preventDefault();
        return;
      }
      if (event.key === "F12") {
        event.preventDefault();
        // Section 15 bindings unchanged (no new shortcut); F12 is a no-op
        // without an OPEN cash session (ADR-025 blocking precondition).
        const cashSession = useCashSessionStore.getState().session;
        const cashOpen = cashSession !== null && cashSession.status === "OPEN";
        if (!modalOpen && cart.lines.length > 0 && !isCreditBlocked() && cashOpen) {
          ui.openModal("checkout");
        }
        return;
      }

      if (inInput) {
        return;
      }
      if (modalOpen) {
        return;
      }

      // Scanner wedge (TASK-DSN-040): a bare printable keystroke outside any
      // text input and outside a modal is redirected into the catalog search
      // bar, so the HENEX HC-3206R (USB-HID wedge) scans hands-free regardless
      // of focus. The first redirect moved focus into the input; subsequent
      // characters arrive with the input focused and take the normal path.
      if (isPrintableKey(event)) {
        const targetInput = document.getElementById(
          SEARCH_INPUT_ID,
        ) as HTMLInputElement | null;
        if (targetInput !== null && !targetInput.disabled) {
          event.preventDefault();
          targetInput.focus();
          targetInput.value = targetInput.value + event.key;
          targetInput.dispatchEvent(new Event("input", { bubbles: true }));
          const length = targetInput.value.length;
          targetInput.setSelectionRange(length, length);
          return;
        }
      }

      if (event.key === "Delete" || event.key === "Backspace") {
        const line = useCartStore.getState().lines[usePosUiStore.getState().selectedLineIndex];
        if (line !== undefined) {
          useCartStore.getState().removeLine(line.key);
          const count = useCartStore.getState().lines.length;
          usePosUiStore.getState().moveSelection(0, count);
          event.preventDefault();
        }
        return;
      }
      if (event.key === "ArrowDown") {
        usePosUiStore.getState().moveSelection(1, useCartStore.getState().lines.length);
        event.preventDefault();
        return;
      }
      if (event.key === "ArrowUp") {
        usePosUiStore.getState().moveSelection(-1, useCartStore.getState().lines.length);
        event.preventDefault();
        return;
      }
      if ((event.ctrlKey || event.metaKey) && (event.key === "k" || event.key === "K")) {
        focusSearch();
        event.preventDefault();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, []);
}
