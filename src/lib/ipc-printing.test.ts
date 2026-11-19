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
  ipcPreviewTsplLabel,
  ipcPreviewTsplStudioLabel,
  ipcPrintTsplStudioLabel,
  ipcUpdatePrinterSettings,
  printingTestPrint,
  printingTestTsplCalibration,
  printingTestTsplLabel,
  printingTestTsplTicket,
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

describe("VT-05 printerTestTsplLabel (TASK-PRT-01)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
    primeSession();
  });
  afterEach(() => {
    clearTauriWindow();
  });

  it("sends session only and maps bytes_sent", async () => {
    invokeMock.mockResolvedValueOnce({ bytes_sent: 347 });
    const result = await printingTestTsplLabel();
    expect(invokeMock).toHaveBeenCalledWith("printer_test_tspl_label", {
      session_token: "tok-idn-1",
      request: { session_token: "tok-idn-1" },
    });
    expect(result).toEqual({ bytesSent: 347 });
  });

  it("a transport error propagates as a typed code", async () => {
    invokeMock.mockRejectedValueOnce("TSPL_TRANSPORT_REQUIRES_SPOOLER");
    await expect(printingTestTsplLabel()).rejects.toBe("TSPL_TRANSPORT_REQUIRES_SPOOLER");
  });
});

describe("VT-07 previewTsplLabel (DOC-03)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
    primeSession();
  });
  afterEach(() => {
    clearTauriWindow();
  });

  it("sends the preview request and maps lines verbatim", async () => {
    invokeMock.mockResolvedValueOnce({
      lines: [
        { slot: "brand", text: "BARAKA", x: 136, y: 2, cell_h: 12 },
        { slot: "name1", text: "JILI ISPANA", x: 8, y: 16, cell_h: 20 },
        { slot: "price", text: "350.00", x: 8, y: 58, cell_h: 48 },
        { slot: "currency", text: "DZD", x: 272, y: 94, cell_h: 12 },
        { slot: "human", text: "AR02", x: 268, y: 146, cell_h: 12 },
      ],
      barcode_x: 8,
      barcode_y: 106,
      barcode_width: 124,
      barcode_height: 38,
      job_len: 361,
    });
    const result = await ipcPreviewTsplLabel("tok-idn-1", {
      name: "JILI ISPANA",
      priceCentimes: "35000",
      barcode: "AR02",
    });
    expect(invokeMock).toHaveBeenCalledWith("preview_tspl_label", {
      session_token: "tok-idn-1",
      request: {
        session_token: "tok-idn-1",
        name: "JILI ISPANA",
        price_centimes: "35000",
        barcode: "AR02",
      },
    });
    expect(result.lines).toHaveLength(5);
    expect(result.lines[2]).toEqual({ slot: "price", text: "350.00", x: 8, y: 58, cellH: 48 });
    expect(result).toMatchObject({
      barcodeX: 8,
      barcodeY: 106,
      barcodeWidth: 124,
      barcodeHeight: 38,
      jobLen: 361,
    });
  });

  it("a validation error propagates as a typed code", async () => {
    invokeMock.mockRejectedValueOnce("TsplBarcodeTooWide");
    await expect(
      ipcPreviewTsplLabel("tok-idn-1", { name: "JILI ISPANA", priceCentimes: "35000", barcode: "WAY-TOO-LONG-12345" }),
    ).rejects.toBe("TsplBarcodeTooWide");
  });
});

describe("VT-06 printerTestTsplTicket (TASK-PRT-03)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
    primeSession();
  });
  afterEach(() => {
    clearTauriWindow();
  });

  it("sends session only and maps bytes_sent", async () => {
    invokeMock.mockResolvedValueOnce({ bytes_sent: 912 });
    const result = await printingTestTsplTicket();
    expect(invokeMock).toHaveBeenCalledWith("printer_test_tspl_ticket", {
      session_token: "tok-idn-1",
      request: { session_token: "tok-idn-1" },
    });
    expect(result).toEqual({ bytesSent: 912 });
  });

  it("a transport error propagates as a typed code", async () => {
    invokeMock.mockRejectedValueOnce("TSPL_TRANSPORT_REQUIRES_SPOOLER");
    await expect(printingTestTsplTicket()).rejects.toBe("TSPL_TRANSPORT_REQUIRES_SPOOLER");
  });
});

describe("VT-08 printerTestTsplCalibration (DOC-06)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
    primeSession();
  });
  afterEach(() => {
    clearTauriWindow();
  });

  it("sends session only and maps bytes_sent", async () => {
    invokeMock.mockResolvedValueOnce({ bytes_sent: 1217 });
    const result = await printingTestTsplCalibration();
    expect(invokeMock).toHaveBeenCalledWith("printer_test_tspl_calibration", {
      session_token: "tok-idn-1",
      request: { session_token: "tok-idn-1" },
    });
    expect(result).toEqual({ bytesSent: 1217 });
  });

  it("a transport error propagates as a typed code", async () => {
    invokeMock.mockRejectedValueOnce("TSPL_TRANSPORT_REQUIRES_SPOOLER");
    await expect(printingTestTsplCalibration()).rejects.toBe(
      "TSPL_TRANSPORT_REQUIRES_SPOOLER",
    );
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
      tspl_offset_x_mm: 1,
      tspl_offset_y_mm: -1,
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
      1,
      -1,
    );
    expect(invokeMock).toHaveBeenCalledWith("printer_update_settings", {
      session_token: "tok-idn-1",
      request: expect.objectContaining({
        tspl_gap_mm: 2,
        tspl_density: 8,
        tspl_speed: 4,
        tspl_offset_x_mm: 1,
        tspl_offset_y_mm: -1,
      }),
    });
    expect(settings.tsplGapMm).toBe(2);
    expect(settings.tsplDensity).toBe(8);
    expect(settings.tsplSpeed).toBe(4);
    expect(settings.tsplOffsetXMm).toBe(1);
    expect(settings.tsplOffsetYMm).toBe(-1);
  });
});

describe("VT-08 printTsplStudioLabel (DSN-107)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
    primeSession();
  });
  afterEach(() => {
    clearTauriWindow();
  });

  it("sends the studio print request with canonical money", async () => {
    invokeMock.mockResolvedValueOnce({ bytes_sent: 412 });
    const bytes = await ipcPrintTsplStudioLabel("tok-idn-1", {
      template: "hardware",
      line1: "CHEVILLE MOLLY M8",
      line2: "PCS",
      line3: "O8X32MM",
      line4: "",
      hidePrice: false,
      priceCentimes: "4500",
      barcode: "HW0001",
      copies: 1,
    });
    expect(invokeMock).toHaveBeenCalledWith("print_tspl_studio_label", {
      session_token: "tok-idn-1",
      request: {
        session_token: "tok-idn-1",
        template: "hardware",
        line1: "CHEVILLE MOLLY M8",
        line2: "PCS",
        line3: "O8X32MM",
        line4: "",
        hide_price: false,
        price_centimes: "4500",
        barcode: "HW0001",
        copies: 1,
      },
    });
    expect(bytes).toBe(412);
  });

  it("rides price null on wholesale hide (textile only)", async () => {
    invokeMock.mockResolvedValueOnce({ bytes_sent: 388 });
    await ipcPrintTsplStudioLabel("tok-idn-1", {
      template: "textile",
      line1: "COTON ML",
      line2: "XL",
      line3: "NOIR",
      line4: "PACK 6 PCS",
      hidePrice: true,
      priceCentimes: null,
      barcode: "TXL001",
      copies: 2,
    });
    expect(invokeMock).toHaveBeenCalledWith(
      "print_tspl_studio_label",
      expect.objectContaining({
        request: expect.objectContaining({ hide_price: true, price_centimes: null }),
      }),
    );
  });

  it("a validation error propagates as a typed code", async () => {
    invokeMock.mockRejectedValueOnce("STUDIO_PRICE_REQUIRED");
    await expect(
      ipcPrintTsplStudioLabel("tok-idn-1", {
        template: "textile",
        line1: "COTON ML",
        line2: "XL",
        line3: "NOIR",
        line4: "PACK 6 PCS",
        hidePrice: false,
        priceCentimes: null,
        barcode: "TXL001",
        copies: 1,
      }),
    ).rejects.toBe("STUDIO_PRICE_REQUIRED");
  });
});

describe("VT-09 previewTsplStudioLabel (DSN-107, DOC-03)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
    primeSession();
  });
  afterEach(() => {
    clearTauriWindow();
  });

  it("sends the studio preview request and maps lines verbatim", async () => {
    invokeMock.mockResolvedValueOnce({
      lines: [
        { slot: "studio_model", text: "COTON ML", x: 16, y: 16, cell_h: 20 },
        { slot: "studio_size", text: "XL", x: 74, y: 40, cell_h: 48 },
        { slot: "studio_color", text: "NOIR", x: 196, y: 44, cell_h: 12 },
        { slot: "studio_pack", text: "PACK 6 PCS", x: 196, y: 60, cell_h: 12 },
        { slot: "studio_price", text: "250.00 DZD", x: 16, y: 92, cell_h: 32 },
      ],
      barcode_x: 59,
      barcode_y: 126,
      barcode_width: 202,
      barcode_height: 28,
      job_len: 402,
    });
    const result = await ipcPreviewTsplStudioLabel("tok-idn-1", {
      template: "textile",
      line1: "COTON ML",
      line2: "XL",
      line3: "NOIR",
      line4: "PACK 6 PCS",
      hidePrice: false,
      priceCentimes: "25000",
      barcode: "TXL001",
    });
    expect(invokeMock).toHaveBeenCalledWith("preview_tspl_studio_label", {
      session_token: "tok-idn-1",
      request: {
        session_token: "tok-idn-1",
        template: "textile",
        line1: "COTON ML",
        line2: "XL",
        line3: "NOIR",
        line4: "PACK 6 PCS",
        hide_price: false,
        price_centimes: "25000",
        barcode: "TXL001",
      },
    });
    expect(result.lines).toHaveLength(5);
    expect(result.lines[1]).toEqual({ slot: "studio_size", text: "XL", x: 74, y: 40, cellH: 48 });
    expect(result).toMatchObject({
      barcodeX: 59,
      barcodeY: 126,
      barcodeWidth: 202,
      barcodeHeight: 28,
      jobLen: 402,
    });
  });
});
