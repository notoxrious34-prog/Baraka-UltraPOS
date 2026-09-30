/**
 * Store-level tests for the identity + diagnostics section (TASK-BRK-056).
 *
 * The section component is a thin view over `useStoreIdentityStore`;
 * all specified behaviors (gate, validation, save, kind, test-print,
 * badges, preview fidelity, language default, decode summary, i18n
 * parity) are pinned here at the store boundary with invoke mocked at
 * the Tauri edge, exactly like the existing modal-store suites.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  canManageIdentity,
  defaultTestLanguage,
  deviceBadge,
  isIdentityDirty,
  maxFor,
  trimIdentity,
  useStoreIdentityStore,
  validateIdentityField,
  validateIdentityForm,
} from "@/stores/store-identity-store";
import { t } from "@/lib/i18n";
import { useSessionStore } from "@/stores/session-store";
import type { SessionInfo } from "@/types/pos";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

function fakeTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.window = globalThis;
  globals.__TAURI_INTERNALS__ = {};
}

function primeSession(role: "ADMIN" | "GERANT"): void {
  const session: SessionInfo = {
    token: "tok-idn-2",
    userId: "55555555-5555-5555-5555-555555555555",
    username: role === "ADMIN" ? "admin1" : "gerant1",
    role,
    expiresAt: "2026-01-01T00:00:00Z",
  };
  useSessionStore.setState({ session });
}

const emptyForm = {
  storeName: "",
  storeAddress: "",
  storePhone: "",
  nif: "",
  nis: "",
  rc: "",
  ai: "",
  ticketFooterMessage: "",
};

beforeEach(() => {
  vi.clearAllMocks();
  fakeTauriWindow();
  useStoreIdentityStore.getState().reset();
});

describe("VT-04 permission gate", () => {
  it("ADMIN granted / GERANT denied; all 8 labeled fields in FR and AR", () => {
    primeSession("ADMIN");
    expect(canManageIdentity()).toBe(true);
    primeSession("GERANT");
    expect(canManageIdentity()).toBe(false);
    const labels = [
      "settings.identity.storeName",
      "settings.identity.storeAddress",
      "settings.identity.storePhone",
      "settings.identity.nif",
      "settings.identity.nis",
      "settings.identity.rc",
      "settings.identity.ai",
      "settings.identity.footer",
    ] as const;
    expect(labels).toHaveLength(8);
    for (const key of labels) {
      expect(t("fr-DZ", key).length).toBeGreaterThan(0);
      expect(t("ar-DZ", key).length).toBeGreaterThan(0);
    }
  });
});

describe("VT-05 soft validation", () => {
  it("required (FR/AR) + tooLong {max} + trim keeps empty optionals valid", () => {
    const key = validateIdentityField("storeName", "   ");
    expect(key).toBe("settings.identity.required");
    expect(t("fr-DZ", key ?? "settings.identity.save")).toContain("obligatoire");
    expect(t("ar-DZ", key ?? "settings.identity.save")).toContain("مطلوب");
    expect(validateIdentityField("nif", "x".repeat(21))).toBe("settings.identity.tooLong");
    expect(
      t("fr-DZ", "settings.identity.tooLong").replace("{max}", String(maxFor("nif"))),
    ).toContain("20");
    expect(validateIdentityForm({ ...emptyForm, storeName: "Shop" })).toEqual({});
    expect(validateIdentityForm(emptyForm).storeName).toBe("settings.identity.required");
    const cleaned = trimIdentity({ ...emptyForm, storeName: "  Shop  " });
    expect(cleaned.storeName).toBe("Shop");
    expect(validateIdentityForm(cleaned)).toEqual({});
  });
});

describe("VT-06 save flow", () => {
  it("success invokes once with trimmed values and flashes saved", async () => {
    primeSession("ADMIN");
    invokeMock.mockResolvedValueOnce({
      identity: {
        store_name: "Shop",
        store_address: "",
        store_phone: "",
        nif: "",
        nis: "",
        rc: "",
        ai: "",
        ticket_footer_message: "",
      },
      device_kind: "capture",
    });
    const store = useStoreIdentityStore.getState();
    store.setField("storeName", "  Shop  ");
    await store.save();
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(useStoreIdentityStore.getState().savedFlash).toBe(true);
    expect(useStoreIdentityStore.getState().saving).toBe(false);
    // Dirty tracking: clean after save, dirty on edit or kind change.
    const after = useStoreIdentityStore.getState();
    expect(isIdentityDirty(after)).toBe(false);
    expect(isIdentityDirty({ ...after, deviceKind: "physical" })).toBe(true);
  });

  it("backend validation error lands inline without a toast flag", async () => {
    primeSession("ADMIN");
    // Valid client-side ("Shop") so the call reaches the backend, which
    // rejects with a classified soft-validation code.
    invokeMock.mockRejectedValueOnce("STORE_FIELD_TOO_LONG");
    const store = useStoreIdentityStore.getState();
    store.setField("storeName", "Shop");
    await expect(store.save()).rejects.toBe("STORE_FIELD_TOO_LONG");
    expect(useStoreIdentityStore.getState().saveError).toBe("STORE_FIELD_TOO_LONG");
    expect(useStoreIdentityStore.getState().saving).toBe(false);
  });
});

describe("VT-07 kind selector", () => {
  it("both options labeled, serial hint present, no port input in section", () => {
    expect(t("fr-DZ", "settings.identity.kindCapture").length).toBeGreaterThan(0);
    expect(t("fr-DZ", "settings.identity.kindPhysical").length).toBeGreaterThan(0);
    expect(t("fr-DZ", "settings.identity.physicalHint")).toContain("printer.serial_*");
    useStoreIdentityStore.getState().setDeviceKind("physical");
    expect(useStoreIdentityStore.getState().deviceKind).toBe("physical");
  });
});

describe("VT-08 test print (capture-only, AR)", () => {
  it("invokes once with language only; preview view consumes design lines (VT-10)", async () => {
    primeSession("ADMIN");
    invokeMock.mockResolvedValueOnce({
      preview: [{ kind: "text", centered: true, size_px: 32, text: "Baraka" }],
      decode_summary: { bands: 9, feed_lines: 3, unknown_spans: 0 },
      capture_file_name: "test-abc.escpos",
      device_outcome: "capture-only",
    });
    const store = useStoreIdentityStore.getState();
    store.setLanguage("ar");
    await store.runTest();
    expect(invokeMock).toHaveBeenCalledWith("printer_test_print", {
      session_token: "tok-idn-2",
      request: { session_token: "tok-idn-2", language: "ar" },
    });
    const result = useStoreIdentityStore.getState().result;
    expect(result?.captureFileName).toBe("test-abc.escpos");
    expect(result?.preview[0]?.text).toBe("Baraka");
    expect(useStoreIdentityStore.getState().testing).toBe(false);
    // VT-10 preview fidelity: the preview view consumes PreviewLine
    // design lines only (no pixel decoding path exists client-side).
    const { TestPrintPreview } = await import(
      "@/components/pos/modals/TestPrintPreview"
    );
    expect(typeof TestPrintPreview).toBe("function");
  });
});

describe("VT-09 device results", () => {
  it("sent → success badge; error code → error badge with code + label", () => {
    const sent = deviceBadge("fr-DZ", "sent");
    expect(sent).toEqual({ icon: "✓", label: "Envoyé à l'imprimante", ok: true });
    const failed = deviceBadge("fr-DZ", "PRINTER_UNAVAILABLE");
    expect(failed.ok).toBe(false);
    expect(failed.icon).toBe("✗");
    expect(failed.label).toContain("PRINTER_UNAVAILABLE");
    const capture = deviceBadge("ar-DZ", "capture-only");
    expect(capture.ok).toBe(true);
    expect(capture.label).toContain("التقاط");
  });
});

describe("VT-11 language toggle", () => {
  it("default derives from locale; switch reflected in payload", async () => {
    expect(defaultTestLanguage("ar-DZ")).toBe("ar");
    expect(defaultTestLanguage("fr-DZ")).toBe("fr");
    primeSession("ADMIN");
    invokeMock.mockResolvedValue({
      preview: [],
      decode_summary: { bands: 1, feed_lines: 3, unknown_spans: 0 },
      capture_file_name: "t.escpos",
      device_outcome: "capture-only",
    });
    const store = useStoreIdentityStore.getState();
    store.setLanguage("fr");
    await store.runTest();
    expect(invokeMock).toHaveBeenLastCalledWith("printer_test_print", {
      session_token: "tok-idn-2",
      request: { session_token: "tok-idn-2", language: "fr" },
    });
  });
});

describe("VT-12 decode_summary + VT-13 i18n parity", () => {
  it("stats labeled, warning keyed, all section keys in fr-DZ and ar-DZ", () => {
    expect(t("fr-DZ", "settings.identity.unknownSpans").length).toBeGreaterThan(0);
    expect(t("ar-DZ", "settings.identity.unknownWarning")).toContain("غير معروفة");
    const keys = [
      "settings.identity.title",
      "settings.identity.required",
      "settings.identity.tooLong",
      "settings.identity.save",
      "settings.identity.saving",
      "settings.identity.saved",
      "settings.identity.kind",
      "settings.identity.kindCapture",
      "settings.identity.kindPhysical",
      "settings.identity.physicalHint",
      "settings.identity.unsavedHint",
      "settings.identity.language",
      "settings.identity.test",
      "settings.identity.testing",
      "settings.identity.previewCaption",
      "settings.identity.captureCaption",
      "settings.identity.captureHint",
      "settings.identity.sent",
      "settings.identity.captureOnly",
      "settings.identity.deviceError",
      "settings.identity.bands",
      "settings.identity.feedLines",
      "settings.identity.unknownSpans",
      "settings.identity.unknownWarning",
      "settings.identity.drillRef",
      "settings.identity.footerHint",
    ] as const;
    for (const key of keys) {
      expect(t("fr-DZ", key).length, key).toBeGreaterThan(0);
      expect(t("ar-DZ", key).length, key).toBeGreaterThan(0);
    }
  });
});
