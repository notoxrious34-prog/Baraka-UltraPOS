/**
 * Framework-free checkout submission controller (TASK-BRK-032).
 *
 * Owns the strict in-flight submission lock and the stable per-session
 * idempotency command id for the checkout modal. The lock flips
 * synchronously BEFORE the dispatch and is released exactly once when the
 * dispatch settles, so repeated triggers (double-tap F12, double-click
 * "Confirmer", mid-flight key storms) can never produce a second
 * concurrent dispatch. The command id is minted once per controller
 * lifetime (= one checkout modal session, since the shell mounts the
 * modal conditionally) and is NEVER regenerated per click — a retry after
 * a failed dispatch reuses the same id, so the backend replays or rejects
 * it as one logical command (Architecture Section 9 / Article 6.2).
 */

import { newCommandId } from "@/lib/money";
import type { CheckoutRequestWire } from "@/lib/ipc";
import type { SaleReceipt } from "@/types/pos";

/** IPC dispatch edge used by the checkout modal. */
export type CheckoutDispatch = (request: CheckoutRequestWire) => Promise<SaleReceipt>;

/** Mirrors the synchronous lock into UI state (React setState or test spy). */
export type OnSubmittingChange = (submitting: boolean) => void;

/** Guarded single-dispatch checkout submission. */
export interface CheckoutSubmission {
  /** Stable idempotency key for this modal session (never per click). */
  readonly commandId: string;
  /** True while the dispatch is in flight. */
  isSubmitting(): boolean;
  /**
   * Guarded confirm: no-op (returns `null`) while a dispatch is already
   * in flight; otherwise flips the lock synchronously, builds the request
   * with the session-stable command id, dispatches exactly once, and
   * returns the in-flight promise. The lock releases when it settles —
   * on failure so the cashier may retry with the SAME id (idempotent
   * replay), on success because the modal has already moved on.
   */
  confirm(buildRequest: (commandId: string) => CheckoutRequestWire): Promise<SaleReceipt> | null;
}

/**
 * Create the checkout submission controller.
 *
 * `onSubmittingChange` mirrors the lock into UI state so the modal can
 * disable its controls; the internal synchronous flag stays authoritative
 * (React state alone cannot block same-tick double triggers).
 */
export function createCheckoutSubmission(
  dispatch: CheckoutDispatch,
  onSubmittingChange: OnSubmittingChange,
): CheckoutSubmission {
  const commandId = newCommandId();
  let submitting = false;
  const notify = (next: boolean): void => {
    submitting = next;
    onSubmittingChange(next);
  };
  return {
    commandId,
    isSubmitting: () => submitting,
    confirm(buildRequest) {
      if (submitting) {
        return null;
      }
      notify(true);
      let dispatched: Promise<SaleReceipt>;
      try {
        dispatched = dispatch(buildRequest(commandId));
      } catch (failure) {
        notify(false);
        return Promise.reject(failure);
      }
      return dispatched.finally(() => {
        notify(false);
      });
    },
  };
}
