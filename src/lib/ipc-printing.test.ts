/**
 * Contract tests for the store-identity + print-diagnostics IPC wrappers
 * (TASK-BRK-056).
 *
 * Pins the exact backend command names (`store_identity_get`,
 * `store_identity_save`, `printer_test_print`), request wire shapes
 * field-for-field, verbatim response mapping, and code-preserving error
 * propagation. The Rust DTO is canonical: wire `device_kind` values
 * "physical" | "capture"; `printer_test_print` carries language only
 * (kind is saved state, not an invocation parameter) [S4].
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  getStoreIdentity,
  ipcUpdatePrinterSettings,
  printingTestPrint,
  saveStoreIdentity,
} from "@/lib/ipc";
import { useSessionStore } from "@/stores/session-store";
import type { SessionInfo } from "@/types/pos";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

/** The bridge probe requires a `window` carrying the Tauri marker. */
function fakeTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.window = globalThis;
  globals.__TAURI_INTERNALS__ = {};
}

function clearTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  delete globals.window;
  delete globals.__TAURI_INTERNALS__;
}

function primeSession(): void {
  const session: SessionInfo = {
    token: "tok-idn-1",
    userId: "55555555-5555-5555-5555-555555555555",
    username: "admin1",
    role: "ADMIN",
    expiresAt: "2026-01-01T00:00:00Z",
    permissions: [],
  };
  useSessionStore.setState({ session });
}

const identityWire = {
  store_name: "Baraka Quincaillerie",
  store_address: "12 Rue des Frères, Alger",
  store_phone: "0550 12 34 56",
  nif: "123456789012345",
  nis: "",
  rc: "16/00-1234567B16",
  ai: "",
  ticket_footer_message: "Merci de votre visite",
};

describe("VT-01 getStoreIdentity", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
    primeSession();
  });
  afterEach(() => {
    clearTauriWindow();
  });

  it("maps full and empty DTOs to typed form state", async () => {
    invokeMock.mockResolvedValueOnce({ identity: identityWire, device_kind: "physical" });
    const state = await getStoreIdentity();
    expect(invokeMock).toHaveBeenCalledWith("store_identity_get", {
      session_token: "tok-idn-1",
    });
    expect(state.identity.storeName).toBe("Baraka Quincaillerie");
    expect(state.identity.ticketFooterMessage).toBe("Merci de votre visite");
    expect(state.deviceKind).toBe("physical");

    invokeMock.mockResolvedValueOnce({
      identity: {
        store_name: "",
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
    const emptyState = await getStoreIdentity();
    expect(Object.values(emptyState.identity).every((value) => value === "")).toBe(true);
    expect(emptyState.deviceKind).toBe("capture");
  });
});

describe("VT-02 saveStoreIdentity", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
    primeSession();
  });
  afterEach(() => {
    clearTauriWindow();
  });

  it("sends the exact command name with trimmed values", async () => {
    invokeMock.mockResolvedValueOnce({ identity: identityWire, device_kind: "capture" });
    const state = await saveStoreIdentity(
      {
        storeName: "  Baraka  ",
        storeAddress: "",
        storePhone: "",
        nif: "",
        nis: "",
        rc: "",
        ai: "",
        ticketFooterMessage: "",
      },
      "capture",
    );
    expect(invokeMock).toHaveBeenCalledWith("store_identity_save", {
      session_token: "tok-idn-1",
      request: {
        session_token: "tok-idn-1",
        identity: {
          store_name: "  Baraka  ",
          store_address: "",
          store_phone: "",
          nif: "",
          nis: "",
          rc: "",
          ai: "",
          ticket_footer_message: "",
        },
        device_kind: "capture",
      },
    });
    expect(state.identity.storeName).toBe("Baraka Quincaillerie");
  });

  it("backend validation rejection propagates as a typed code", async () => {
    invokeMock.mockRejectedValueOnce("STORE_NAME_REQUIRED");
    await expect(
      saveStoreIdentity(
        {
          storeName: "",
          storeAddress: "",
          storePhone: "",
          nif: "",
          nis: "",
          rc: "",
          ai: "",
          ticketFooterMessage: "",
        },
        "physical",
      ),
    ).rejects.toBe("STORE_NAME_REQUIRED");
  });
});

describe("VT-03 printingTestPrint", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
    primeSession();
  });
  afterEach(() => {
    clearTauriWindow();
  });

  it("types a capture-only result end-to-end (language only on the wire)", async () => {
    invokeMock.mockResolvedValueOnce({
      preview: [
        { kind: "text", centered: true, size_px: 32, text: "Baraka" },
        { kind: "rule", centered: true, size_px: 0, text: "" },
      ],
      decode_summary: { bands: 12, feed_lines: 3, unknown_spans: 0 },
      capture_file_name: "test-20260930T120000.000Z-1234.escpos",
      device_outcome: "capture-only",
    });
    const result = await printingTestPrint("ar");
    expect(invokeMock).toHaveBeenCalledWith("printer_test_print", {
      session_token: "tok-idn-1",
      request: { session_token: "tok-idn-1", language: "ar" },
    });
    expect(result.preview).toHaveLength(2);
    expect(result.preview[0]).toEqual({
      kind: "text",
      centered: true,
      sizePx: 32,
      text: "Baraka",
    });
    expect(result.decodeSummary).toEqual({ bands: 12, feedLines: 3, unknownSpans: 0 });
    expect(result.captureFileName).toContain(".escpos");
    expect(result.deviceOutcome).toBe("capture-only");
  });

  it("a classified device error code propagates as a typed error", async () => {
    invokeMock.mockRejectedValueOnce("PRINTER_UNAVAILABLE");
    await expect(printingTestPrint("fr")).rejects.toBe("PRINTER_UNAVAILABLE");
  });
});

describe("VT-04 updatePrinterSettings TSPL media (LIVE-2026-11-06)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
    primeSession();
  });
  afterEach(() => {
    clearTauriWindow();
  });

  it("passes gap/density/speed on the wire and maps the stored media back", async () => {
    invokeMock.mockResolvedValueOnce({
      serial_port: "",
      serial_baud: 9600,
      paper_width_dots: 576,
      paper_width_mm: 80,
      transport: "spooler",
      queue_name: "XP-410B",
      ticket_render_mode: "escpos",
      tspl_probe_mode: "normal",
      tspl_gap_mm: 2,
      tspl_density: 8,
      tspl_speed: 4,
    });
    const settings = await ipcUpdatePrinterSettings(
      "tok-idn-1",
      "",
      9600,
      80,
      "spooler",
      "XP-410B",
      "escpos",
      "normal",
      2,
      8,
      4,
    );
    expect(invokeMock).toHaveBeenCalledWith("printer_update_settings", {
      session_token: "tok-idn-1",
      request: expect.objectContaining({
        tspl_gap_mm: 2,
        tspl_density: 8,
        tspl_speed: 4,
      }),
    });
    expect(settings.tsplGapMm).toBe(2);
    expect(settings.tsplDensity).toBe(8);
    expect(settings.tsplSpeed).toBe(4);
  });
});
