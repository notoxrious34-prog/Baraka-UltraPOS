/**
 * Cost/Margin Privacy Shield (TASK-DSN-103): counter-eyes protection.
 *
 * Session-only boolean (no persistence — every launch starts unshielded,
 * same convention as ADR-069): while shielded, margin/cost figures render
 * masked (`•••`) instead of their figures. Toggled by `Ctrl+H` at the
 * counter (CartPanel) or the shield chip beside the document switcher.
 */

import { create } from "zustand";

interface PrivacyShieldState {
  /** True while cost/margin figures stay masked from counter eyes. */
  shielded: boolean;
  toggleShield: () => void;
  setShielded: (shielded: boolean) => void;
}

export const usePrivacyShieldStore = create<PrivacyShieldState>()((set) => ({
  shielded: false,
  toggleShield: () => {
    set((state) => ({ shielded: !state.shielded }));
  },
  setShielded: (shielded: boolean) => {
    set({ shielded });
  },
}));
