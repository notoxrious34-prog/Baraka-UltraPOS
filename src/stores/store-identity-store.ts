/**
 * Store identity + print diagnostics store (TASK-BRK-056).
 *
 * Length caps mirror the domain `identity_caps` exactly
 * (crates/domain/src/printing/receipt_model.rs): name 80, address 120,
 * phone 40, fiscal identifiers 20, footer 160 chars / 3 lines. The
 * directive table (address 20, phone 20) diverges from the Rust DTO —
 * the Rust DTO is canonical [S1].
 *
 * The test-print kind is SAVED state, not an invocation parameter: the
 * backend `printer_test_print` request carries only `{ language }`
 * (Rust DTO canonical [S4]). The kind selector writes through
 * `saveStoreIdentity` like the rest of the form.
 */

import { create } from "zustand";
import {
  getStoreIdentity,
  printingTestPrint,
  saveStoreIdentity,
  type DeviceKind,
  type StoreIdentity,
  type TestPrintLanguage,
  type TestPrintResult,
} from "@/lib/ipc";
import { t, type I18nKey } from "@/lib/i18n";
import { useSessionStore } from "@/stores/session-store";
import type { Locale } from "@/types/pos";

/** Field caps (domain `identity_caps`, mirrored exactly). */
export const IDENTITY_CAPS = {
  storeName: 80,
  storeAddress: 120,
  storePhone: 40,
  fiscalId: 20,
  footerMessage: 160,
  footerLines: 3,
} as const;

const EMPTY_IDENTITY: StoreIdentity = {
  storeName: "",
  storeAddress: "",
  storePhone: "",
  nif: "",
  nis: "",
  rc: "",
  ai: "",
  ticketFooterMessage: "",
};

/** Per-field maximum length. */
export function maxFor(field: keyof StoreIdentity): number {
  switch (field) {
    case "storeName":
      return IDENTITY_CAPS.storeName;
    case "storeAddress":
      return IDENTITY_CAPS.storeAddress;
    case "storePhone":
      return IDENTITY_CAPS.storePhone;
    case "ticketFooterMessage":
      return IDENTITY_CAPS.footerMessage;
    default:
      return IDENTITY_CAPS.fiscalId;
  }
}

/**
 * Validate one field value (untrimmed input accepted; length counted in
 * chars so Arabic values are not penalized for multi-byte encoding).
 * Returns the i18n key of the violation, or null when valid.
 */
export function validateIdentityField(
  field: keyof StoreIdentity,
  value: string,
): I18nKey | null {
  if (field === "storeName" && value.trim() === "") {
    return "settings.identity.required";
  }
  if ([...value].length > maxFor(field)) {
    return "settings.identity.tooLong";
  }
  return null;
}

/** Validate the whole form; returns per-field message keys. */
export function validateIdentityForm(
  identity: StoreIdentity,
): Partial<Record<keyof StoreIdentity, I18nKey>> {
  const errors: Partial<Record<keyof StoreIdentity, I18nKey>> = {};
  (Object.keys(EMPTY_IDENTITY) as Array<keyof StoreIdentity>).forEach((field) => {
    const violation = validateIdentityField(field, identity[field]);
    if (violation !== null) {
      errors[field] = violation;
    }
  });
  return errors;
}

/** Trim every field (defense in depth; the backend trims as well). */
export function trimIdentity(identity: StoreIdentity): StoreIdentity {
  return {
    storeName: identity.storeName.trim(),
    storeAddress: identity.storeAddress.trim(),
    storePhone: identity.storePhone.trim(),
    nif: identity.nif.trim(),
    nis: identity.nis.trim(),
    rc: identity.rc.trim(),
    ai: identity.ai.trim(),
    ticketFooterMessage: identity.ticketFooterMessage.trim(),
  };
}

/** Default test-print language derived from the UI locale. */
export function defaultTestLanguage(locale: Locale): TestPrintLanguage {
  return locale === "ar-DZ" ? "ar" : "fr";
}

/** Client-side gate mirror (UX only; Rust remains authoritative). */
export function canManageIdentity(): boolean {
  return useSessionStore.getState().session?.role === "ADMIN";
}

/** Badge derivation from a device outcome (icon + text; never color alone). */
export function deviceBadge(
  locale: Locale,
  outcome: string,
): { icon: "✓" | "✗"; label: string; ok: boolean } {
  if (outcome === "sent") {
    return { icon: "✓", label: t(locale, "settings.identity.sent"), ok: true };
  }
  if (outcome === "capture-only") {
    return { icon: "✓", label: t(locale, "settings.identity.captureOnly"), ok: true };
  }
  return {
    icon: "✗",
    label: `${t(locale, "settings.identity.deviceError")}: ${outcome}`,
    ok: false,
  };
}

interface StoreIdentityState {
  form: StoreIdentity;
  saved: StoreIdentity;
  deviceKind: DeviceKind;
  savedDeviceKind: DeviceKind;
  language: TestPrintLanguage;
  fieldErrors: Partial<Record<keyof StoreIdentity, I18nKey>>;
  saveError: string | null;
  savedFlash: boolean;
  loading: boolean;
  saving: boolean;
  testing: boolean;
  result: TestPrintResult | null;
  testError: string | null;
  load: (locale: Locale) => Promise<void>;
  setField: (field: keyof StoreIdentity, value: string) => void;
  blurField: (field: keyof StoreIdentity) => void;
  setDeviceKind: (kind: DeviceKind) => void;
  setLanguage: (language: TestPrintLanguage) => void;
  save: () => Promise<void>;
  runTest: () => Promise<void>;
  reset: () => void;
}

const initialState = {
  form: { ...EMPTY_IDENTITY },
  saved: { ...EMPTY_IDENTITY },
  deviceKind: "capture" as DeviceKind,
  savedDeviceKind: "capture" as DeviceKind,
  language: "fr" as TestPrintLanguage,
  fieldErrors: {},
  saveError: null,
  savedFlash: false,
  loading: false,
  saving: false,
  testing: false,
  result: null,
  testError: null,
};

export const useStoreIdentityStore = create<StoreIdentityState>()((set, get) => ({
  ...initialState,

  load: async (locale) => {
    if (get().loading) {
      return;
    }
    set({ loading: true });
    try {
      const state = await getStoreIdentity();
      set({
        form: { ...state.identity },
        saved: { ...state.identity },
        deviceKind: state.deviceKind,
        savedDeviceKind: state.deviceKind,
        language: defaultTestLanguage(locale),
        fieldErrors: validateIdentityForm(state.identity),
        loading: false,
      });
    } catch (error: unknown) {
      set({ loading: false });
      throw error;
    }
  },

  setField: (field, value) => {
    const form = { ...get().form, [field]: value };
    set({ form, savedFlash: false });
  },

  blurField: (field) => {
    const violation = validateIdentityField(field, get().form[field]);
    const fieldErrors = { ...get().fieldErrors };
    if (violation === null) {
      delete fieldErrors[field];
    } else {
      fieldErrors[field] = violation;
    }
    set({ fieldErrors });
  },

  setDeviceKind: (kind) => {
    set({ deviceKind: kind });
  },

  setLanguage: (language) => {
    set({ language });
  },

  save: async () => {
    if (get().saving) {
      return;
    }
    const cleaned = trimIdentity(get().form);
    const fieldErrors = validateIdentityForm(cleaned);
    set({ fieldErrors, saveError: null, savedFlash: false });
    if (Object.keys(fieldErrors).length > 0) {
      return;
    }
    set({ saving: true });
    let alive = true;
    try {
      const state = await saveStoreIdentity(cleaned, get().deviceKind);
      if (!alive) {
        return;
      }
      set({
        form: { ...state.identity },
        saved: { ...state.identity },
        savedDeviceKind: state.deviceKind,
        saving: false,
        savedFlash: true,
      });
    } catch (error: unknown) {
      if (!alive) {
        return;
      }
      // Backend soft-validation failures surface inline (no toast);
      // unexpected failures use the generic error pattern.
      const code = typeof error === "string" ? error : "INTERNAL_ERROR";
      if (code === "STORE_NAME_REQUIRED" || code === "STORE_FIELD_TOO_LONG") {
        const fieldErrors: Partial<Record<keyof StoreIdentity, I18nKey>> =
          code === "STORE_NAME_REQUIRED"
            ? { storeName: "settings.identity.required" }
            : { ...get().fieldErrors };
        set({ fieldErrors, saveError: code, saving: false });
      } else {
        set({ saveError: code, saving: false });
      }
      throw error;
    } finally {
      alive = false;
    }
  },

  runTest: async () => {
    if (get().testing) {
      return;
    }
    set({ testing: true, testError: null });
    let alive = true;
    try {
      const result = await printingTestPrint(get().language);
      if (!alive) {
        return;
      }
      set({ result, testing: false });
    } catch (error: unknown) {
      if (!alive) {
        return;
      }
      const code = typeof error === "string" ? error : "INTERNAL_ERROR";
      set({ testError: code, testing: false });
      throw error;
    } finally {
      alive = false;
    }
  },

  reset: () => {
    set({ ...initialState, form: { ...EMPTY_IDENTITY }, saved: { ...EMPTY_IDENTITY } });
  },
}));

/** True when the form differs from the last saved state (or kind changed). */
export function isIdentityDirty(state: {
  form: StoreIdentity;
  saved: StoreIdentity;
  deviceKind: DeviceKind;
  savedDeviceKind: DeviceKind;
}): boolean {
  return (
    state.deviceKind !== state.savedDeviceKind ||
    (Object.keys(state.form) as Array<keyof StoreIdentity>).some(
      (field) => state.form[field] !== state.saved[field],
    )
  );
}
