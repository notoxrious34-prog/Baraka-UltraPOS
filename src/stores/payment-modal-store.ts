/**
 * Payment modal target store (TASK-BRK-037).
 *
 * Holds the customer payload for the globally-mounted RecordPaymentModal
 * (rendered by PosShell). Balances stored here are backend-sourced
 * snapshots fetched fresh at open time — never computed locally.
 */

import { create } from "zustand";

export interface PaymentTarget {
  readonly customerId: string;
  readonly customerName: string;
  readonly balanceCentimes: string;
  /** Modal to restore when the payment flow closes (e.g. "customer"). */
  readonly returnTo: "customer" | null;
}

interface PaymentModalState {
  readonly target: PaymentTarget | null;
  openPayment: (target: PaymentTarget) => void;
  clearPayment: () => void;
}

export const usePaymentModalStore = create<PaymentModalState>()((set) => ({
  target: null,
  openPayment: (target) => set({ target }),
  clearPayment: () => set({ target: null }),
}));
