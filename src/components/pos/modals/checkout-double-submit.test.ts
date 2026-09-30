/**
 * Double-submit guard tests for the checkout submission controller
 * (TASK-BRK-032).
 *
 * Drives the EXACT controller `CheckoutModal` consumes
 * (`createCheckoutSubmission(ipcCheckout, setIsSubmitting)`): proves the
 * single-dispatch invariant (repeated F12/Enter/confirms while a request
 * is in flight dispatch the IPC bridge exactly ONCE) and the stable
 * per-session command id (minted at modal mount, never regenerated per
 * click; a retry after failure reuses it as one logical backend command).
 */

import { describe, expect, it, vi } from "vitest";
import { createCheckoutSubmission } from "@/components/pos/modals/checkout-submission";
import type { CheckoutRequestWire } from "@/lib/ipc";
import type { SaleReceipt } from "@/types/pos";

/** Minimal valid checkout request carrying the given command id. */
function requestFor(commandId: string): CheckoutRequestWire {
  return {
    command_id: commandId,
    session_token: "tok-checkout-1",
    warehouse_id: "wh-1",
    customer_id: null,
    document_type: "TICKET",
    document_language: "FR",
    lines: [
      {
        product_id: "f20f6f21-0000-4000-8000-000000000000",
        unit_id: "78d65d76-0000-4000-8000-000000000000",
        quantity_micro: "10000",
        unit_price_centimes: null,
        line_discount_centimes: "0",
      },
    ],
    global_discount_centimes: "0",
    payments: [{ payment_method: "CASH", amount_centimes: "30050" }],
    allow_negative_stock: false,
    notes: null,
  };
}

/** Committed receipt shape (TICK-2026-000002 figures). */
function receiptFor(documentId: string): SaleReceipt {
  return {
    document_id: documentId,
    document_number: "TICK-2026-000002",
    subtotal_ht_centimes: "15050",
    tva_total_centimes: "0",
    discount_total_centimes: "0",
    timbre_centimes: "500",
    total_ttc_centimes: "15550",
    paid_amount_centimes: "30050",
    change_amount_centimes: "14500",
    print_job_id: "job-1",
  };
}

/** Deferred dispatch: records every call, settles on demand. */
function deferredDispatch() {
  const calls: CheckoutRequestWire[] = [];
  let settle!: (receipt: SaleReceipt) => void;
  let fail!: (reason: unknown) => void;
  const dispatch = vi.fn((request: CheckoutRequestWire) => {
    calls.push(request);
    return new Promise<SaleReceipt>((resolve, reject) => {
      settle = resolve;
      fail = reject;
    });
  });
  return {
    dispatch,
    calls,
    settleWith: (receipt: SaleReceipt): void => {
      settle(receipt);
    },
    failWith: (reason: unknown): void => {
      fail(reason);
    },
  };
}

describe("createCheckoutSubmission (CheckoutModal's submission core)", () => {
  it("dispatches exactly once while a request is in flight (double-tap guard)", async () => {
    const { dispatch, settleWith } = deferredDispatch();
    const submission = createCheckoutSubmission(dispatch, () => undefined);
    const buildRequest = vi.fn((commandId: string) => requestFor(commandId));
    // Triple trigger in the same tick (double-tap F12 + double-click).
    const first = submission.confirm(buildRequest);
    const second = submission.confirm(buildRequest);
    const third = submission.confirm(buildRequest);
    expect(first).not.toBeNull();
    expect(second).toBeNull();
    expect(third).toBeNull();
    if (first === null) {
      throw new Error("first trigger must dispatch");
    }
    expect(dispatch).toHaveBeenCalledTimes(1);
    expect(buildRequest).toHaveBeenCalledTimes(1);
    expect(submission.isSubmitting()).toBe(true);
    settleWith(receiptFor("doc-1"));
    const receipt = await first;
    expect(receipt.document_number).toBe("TICK-2026-000002");
    expect(submission.isSubmitting()).toBe(false);
    expect(dispatch).toHaveBeenCalledTimes(1);
  });

  it("keeps one command_id across the modal session and reuses it on retry", async () => {
    const { dispatch, calls, failWith, settleWith } = deferredDispatch();
    const submission = createCheckoutSubmission(dispatch, () => undefined);
    expect(submission.commandId).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
    );
    // First attempt fails (e.g. transient IPC error): the lock releases.
    const first = submission.confirm((commandId) => requestFor(commandId));
    expect(first).not.toBeNull();
    if (first === null) {
      throw new Error("first attempt must dispatch");
    }
    failWith("INTERNAL_ERROR");
    await expect(first).rejects.toBe("INTERNAL_ERROR");
    expect(submission.isSubmitting()).toBe(false);
    // Retry inside the SAME modal session: the SAME logical command id —
    // the backend replays or rejects it, never books it twice.
    const retry = submission.confirm((commandId) => requestFor(commandId));
    expect(retry).not.toBeNull();
    if (retry === null) {
      throw new Error("retry must dispatch after failure release");
    }
    expect(dispatch).toHaveBeenCalledTimes(2);
    const [firstCall, retryCall] = calls;
    if (firstCall === undefined || retryCall === undefined) {
      throw new Error("both attempts must have dispatched");
    }
    expect(retryCall.command_id).toBe(firstCall.command_id);
    expect(retryCall.command_id).toBe(submission.commandId);
    settleWith(receiptFor("doc-retry"));
    await retry;
    expect(calls).toHaveLength(2);
  });

  it("mirrors the lock into UI state exactly around the in-flight window", async () => {
    const { dispatch, settleWith } = deferredDispatch();
    const states: boolean[] = [];
    const submission = createCheckoutSubmission(dispatch, (submitting) => {
      states.push(submitting);
    });
    const first = submission.confirm((commandId) => requestFor(commandId));
    expect(submission.isSubmitting()).toBe(true);
    settleWith(receiptFor("doc-1"));
    await first;
    expect(submission.isSubmitting()).toBe(false);
    expect(states).toEqual([true, false]);
  });

  it("releases the lock and rejects when the dispatch throws synchronously", async () => {
    const dispatch = vi.fn(() => {
      throw new Error("BRIDGE_UNAVAILABLE");
    });
    const states: boolean[] = [];
    const submission = createCheckoutSubmission(dispatch, (submitting) => {
      states.push(submitting);
    });
    const attempt = submission.confirm((commandId) => requestFor(commandId));
    await expect(attempt).rejects.toThrow("BRIDGE_UNAVAILABLE");
    expect(submission.isSubmitting()).toBe(false);
    expect(states).toEqual([true, false]);
  });
});
