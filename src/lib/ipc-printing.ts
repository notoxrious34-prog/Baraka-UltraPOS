/**
 * Printing bridge: print outbox, silent TSPL jobs, printer/transport
 * settings, fiscal toggle, identity + diagnostics, vertical profile.
 * Split from `ipc.ts` (Wave 3); byte-identical motion.
 */

import type { VerticalProfileInfo } from "@/types/pos";
import { call } from "./ipc-kernel";

export interface PrintStatus {
  jobId: string;
  status: string;
  attemptCount: string;
}

/** Latest outbox state for a document (`NONE` when absent). */
export async function ipcGetPrintStatus(
  sessionToken: string,
  documentId: string,
): Promise<PrintStatus> {
  interface StatusWire {
    job_id: string;
    status: string;
    attempt_count: string;
  }
  const wire = await call<StatusWire>("documents_get_print_status", {
    session_token: sessionToken,
    document_id: documentId,
  });
  return {
    jobId: wire.job_id,
    status: wire.status,
    attemptCount: wire.attempt_count,
  };
}

/** Enqueue a watermarked DUPLICATA reprint; resolves the new job id. */
export async function ipcReprint(
  sessionToken: string,
  documentId: string,
): Promise<string> {
  interface ReprintWire {
    job_id: string;
    document_id: string;
  }
  const wire = await call<ReprintWire>("documents_reprint", {
    request: { session_token: sessionToken, document_id: documentId },
  });
  return wire.job_id;
}

/** Silent 40x20 label job input (mirrors TsplLabelRequest; copies 1..99 validated upstream). */
export interface TsplLabelJob {
  name: string;
  priceCentimes: string;
  barcode: string;
  copies: number;
}

/** Silent TSPL print outcome: spooled byte count (mirrors TsplPrintResponse). */
export interface TsplPrintOutcome {
  bytesSent: number;
}

interface TsplPrintWire {
  bytes_sent: number;
}

/** Silent 40x20 mm shelf-label print (button-triggered, spooler only — never a dialog). */
export async function ipcPrintTsplLabel(
  sessionToken: string,
  job: TsplLabelJob,
): Promise<number> {
  const wire = await call<TsplPrintWire>("print_tspl_label", {
    request: {
      session_token: sessionToken,
      name: job.name,
      price_centimes: job.priceCentimes,
      barcode: job.barcode,
      copies: job.copies,
    },
  });
  return wire.bytes_sent;
}

/** Engine 40x20 label preview input (mirrors TsplLabelPreviewRequest; price as canonical centime string). */
export interface TsplLabelPreviewJob {
  name: string;
  priceCentimes: string;
  barcode: string;
}

/** One engine preview line (mirrors TsplPreviewLineDto). */
export interface TsplPreviewLine {
  slot: string;
  text: string;
  x: number;
  y: number;
  cellH: number;
}

/** Engine 40x20 label preview (mirrors TsplLabelPreviewResponse). */
export interface TsplLabelPreview {
  lines: TsplPreviewLine[];
  barcodeX: number;
  barcodeY: number;
  barcodeWidth: number;
  barcodeHeight: number;
  jobLen: number;
}

interface TsplPreviewLineWire {
  slot: string;
  text: string;
  x: number;
  y: number;
  cell_h: number;
}

interface TsplLabelPreviewWire {
  lines: TsplPreviewLineWire[];
  barcode_x: number;
  barcode_y: number;
  barcode_width: number;
  barcode_height: number;
  job_len: number;
}

/** Read-only engine plan for the 40x20 label (DOC-03) — the modal renders these lines verbatim. */
export async function ipcPreviewTsplLabel(
  sessionToken: string,
  job: TsplLabelPreviewJob,
): Promise<TsplLabelPreview> {
  const wire = await call<TsplLabelPreviewWire>("preview_tspl_label", {
    request: {
      session_token: sessionToken,
      name: job.name,
      price_centimes: job.priceCentimes,
      barcode: job.barcode,
    },
  });
  return {
    lines: wire.lines.map((line) => ({
      slot: line.slot,
      text: line.text,
      x: line.x,
      y: line.y,
      cellH: line.cell_h,
    })),
    barcodeX: wire.barcode_x,
    barcodeY: wire.barcode_y,
    barcodeWidth: wire.barcode_width,
    barcodeHeight: wire.barcode_height,
    jobLen: wire.job_len,
  };
}

/** Label Studio template selector (mirrors the StudioTemplate wire spelling). */
export type StudioTemplate = 'textile' | 'hardware';

/**
 * Label Studio print input (mirrors StudioLabelRequest). Money never
 * travels as a float: `priceCentimes` is a canonical centime string,
 * `null` = wholesale price-hide (textile only — the backend rejects a
 * shown price + hide flag, or a missing price, loudly).
 */
export interface StudioLabelJob {
  template: StudioTemplate;
  line1: string;
  line2: string;
  line3: string;
  line4: string;
  hidePrice: boolean;
  priceCentimes: string | null;
  barcode: string;
  copies: number;
}

/** Silent studio-template print to the LABEL role (button-triggered, spooler only). */
export async function ipcPrintTsplStudioLabel(
  sessionToken: string,
  job: StudioLabelJob,
): Promise<number> {
  const wire = await call<TsplPrintWire>('print_tspl_studio_label', {
    request: {
      session_token: sessionToken,
      template: job.template,
      line1: job.line1,
      line2: job.line2,
      line3: job.line3,
      line4: job.line4,
      hide_price: job.hidePrice,
      price_centimes: job.priceCentimes,
      barcode: job.barcode,
      copies: job.copies,
    },
  });
  return wire.bytes_sent;
}

/**
 * Label Studio preview input (mirrors StudioLabelPreviewRequest —
 * same fields as the print job minus copies, single-copy preview).
 */
export interface StudioLabelPreviewJob {
  template: StudioTemplate;
  line1: string;
  line2: string;
  line3: string;
  line4: string;
  hidePrice: boolean;
  priceCentimes: string | null;
  barcode: string;
}

/** Read-only studio plan (DOC-03) — the modal renders these lines verbatim. */
export async function ipcPreviewTsplStudioLabel(
  sessionToken: string,
  job: StudioLabelPreviewJob,
): Promise<TsplLabelPreview> {
  const wire = await call<TsplLabelPreviewWire>('preview_tspl_studio_label', {
    request: {
      session_token: sessionToken,
      template: job.template,
      line1: job.line1,
      line2: job.line2,
      line3: job.line3,
      line4: job.line4,
      hide_price: job.hidePrice,
      price_centimes: job.priceCentimes,
      barcode: job.barcode,
    },
  });
  return {
    lines: wire.lines.map((line) => ({
      slot: line.slot,
      text: line.text,
      x: line.x,
      y: line.y,
      cellH: line.cell_h,
    })),
    barcodeX: wire.barcode_x,
    barcodeY: wire.barcode_y,
    barcodeWidth: wire.barcode_width,
    barcodeHeight: wire.barcode_height,
    jobLen: wire.job_len,
  };
}

/** Silent 80 mm ticket print (button-triggered, spooler only — never a dialog). */
export async function ipcPrintTsplTicket(
  sessionToken: string,
  documentId: string,
  isReprint: boolean,
): Promise<number> {
  const wire = await call<TsplPrintWire>("print_tspl_ticket", {
    request: { session_token: sessionToken, document_id: documentId, is_reprint: isReprint },
  });
  return wire.bytes_sent;
}

/** Silent raster-only print (BRK-101 — TEXT path DEAD; WYSIWYG pixels). */
export async function ipcPrintRawBitmap(
  sessionToken: string,
  pixels: Uint8Array,
  widthBytes: number,
  heightRows: number,
  copies: number,
): Promise<number> {
  const wire = await call<TsplPrintWire>("print_raw_bitmap", {
    request: {
      session_token: sessionToken,
      // Vec<u8> rides a JSON number array; Array.from keeps bytes unsigned.
      pixels: Array.from(pixels),
      width_bytes: widthBytes,
      height_rows: heightRows,
      copies,
    },
  });
  return wire.bytes_sent;
}

/** Active printer transport arm (mirrors printer.transport). */
export type PrinterTransport = 'serial' | 'spooler';

/** Ticket render mode (mirrors printer.ticket_render_mode, ADR-104). */
export type TicketRenderMode = 'escpos' | 'graphique';

/**
 * TSPL diagnostic probe mode (BRK-100 TEMPORARY-DIAGNOSTIC wire spelling;
 * remove with the switch in the lock-in task). Only these four spellings are
 * ever constructed or sent — invalid values never reach the IPC layer.
 */
export type TsplProbeMode = 'normal' | 'no_codepage' | 'ascii_only' | 'font1_only';

/** Parse unknown wire input; unknown spellings fall back to `normal`. */
function toTsplProbeMode(raw: unknown): TsplProbeMode {
  return raw === 'no_codepage' || raw === 'ascii_only' || raw === 'font1_only'
    ? raw
    : 'normal';
}

export interface PrinterSettings {
  serialPort: string;
  serialBaud: number;
  paperWidthDots: number;
  paperWidthMm: number;
  transport: PrinterTransport;
  queueName: string;
  /** Receipt-role arm (BRK-113): receipts/reports ride this transport. */
  receiptTransport: PrinterTransport;
  /** Receipt-role primary spooler queue. */
  receiptQueueName: string;
  /** Receipt-role fallback queue ("" = no fallback). */
  receiptQueueNameFallback: string;
  /** Label-role arm (BRK-113): 40x20 labels ride this transport. */
  labelTransport: PrinterTransport;
  /** Label-role primary spooler queue. */
  labelQueueName: string;
  /** Label-role fallback queue ("" = no fallback). */
  labelQueueNameFallback: string;
  ticketRenderMode: TicketRenderMode;
  /** TEMPORARY-DIAGNOSTIC probe mode (BRK-100 — remove in lock-in). */
  tsplProbeMode: TsplProbeMode;
  /** TSPL media (LIVE-2026-11-06): gap mm 0..10, density 0..15, speed 1..6. */
  tsplGapMm: number;
  tsplDensity: number;
  tsplSpeed: number;
  /** DOC-06 calibration shift in mm, -5..=+5 per axis (default 0). */
  tsplOffsetXMm: number;
  tsplOffsetYMm: number;
}

function toTicketRenderMode(raw: unknown): TicketRenderMode {
  return raw === 'graphique' ? 'graphique' : 'escpos';
}

/** Parse a role arm: receipt falls back to serial, label to spooler. */
function toRoleTransport(raw: unknown, fallback: PrinterTransport): PrinterTransport {
  if (raw === 'spooler') return 'spooler';
  if (raw === 'serial') return 'serial';
  return fallback;
}

function toQueueName(raw: unknown): string {
  return typeof raw === 'string' ? raw : '';
}

/** Read serial printer configuration. */
export async function ipcGetPrinterSettings(sessionToken: string): Promise<PrinterSettings> {
  interface SettingsWire {
    serial_port: string;
    serial_baud: number;
    paper_width_dots: number;
    paper_width_mm: number;
    transport?: string;
    queue_name?: string;
    tspl_gap_mm?: number;
    tspl_density?: number;
    tspl_speed?: number;
    tspl_offset_x_mm?: number;
    tspl_offset_y_mm?: number;
  }
  const wire = await call<SettingsWire>("printer_get_settings", {
    session_token: sessionToken,
  });
  return {
    serialPort: wire.serial_port,
    serialBaud: wire.serial_baud,
    paperWidthDots: wire.paper_width_dots,
    paperWidthMm: wire.paper_width_mm,
    transport: wire.transport === 'spooler' ? 'spooler' : 'serial',
    queueName: typeof wire.queue_name === 'string' ? wire.queue_name : '',
    receiptTransport: toRoleTransport(
      (wire as { receipt_transport?: unknown }).receipt_transport,
      'serial',
    ),
    receiptQueueName: toQueueName(
      (wire as { receipt_queue_name?: unknown }).receipt_queue_name,
    ),
    receiptQueueNameFallback: toQueueName(
      (wire as { receipt_queue_name_fallback?: unknown }).receipt_queue_name_fallback,
    ),
    labelTransport: toRoleTransport(
      (wire as { label_transport?: unknown }).label_transport,
      'spooler',
    ),
    labelQueueName: toQueueName(
      (wire as { label_queue_name?: unknown }).label_queue_name,
    ),
    labelQueueNameFallback: toQueueName(
      (wire as { label_queue_name_fallback?: unknown }).label_queue_name_fallback,
    ),
    ticketRenderMode: toTicketRenderMode(
      (wire as { ticket_render_mode?: unknown }).ticket_render_mode,
    ),
    tsplProbeMode: toTsplProbeMode(
      (wire as { tspl_probe_mode?: unknown }).tspl_probe_mode,
    ),
    tsplGapMm: typeof wire.tspl_gap_mm === "number" ? wire.tspl_gap_mm : 3,
    tsplDensity: typeof wire.tspl_density === "number" ? wire.tspl_density : 8,
    tsplSpeed: typeof wire.tspl_speed === "number" ? wire.tspl_speed : 4,
    tsplOffsetXMm:
      typeof wire.tspl_offset_x_mm === "number" ? wire.tspl_offset_x_mm : 0,
    tsplOffsetYMm:
      typeof wire.tspl_offset_y_mm === "number" ? wire.tspl_offset_y_mm : 0,
  };
}

/** Persist serial port + baud + paper width + transport + render mode + TEMPORARY probe mode (ADMIN only server-side). */
export async function ipcUpdatePrinterSettings(
  sessionToken: string,
  serialPort: string,
  serialBaud: number,
  paperWidthMm: number,
  transport: PrinterTransport = 'serial',
  queueName = '',
  ticketRenderMode?: TicketRenderMode | null,
  // TEMPORARY-DIAGNOSTIC (BRK-100 — remove in lock-in): null/absent keeps stored mode.
  tsplProbeMode?: TsplProbeMode | null,
  // TSPL media (LIVE-2026-11-06): null/absent keeps the stored value.
  tsplGapMm?: number | null,
  tsplDensity?: number | null,
  tsplSpeed?: number | null,
  // DOC-06 calibration: null/absent keeps the stored shift.
  tsplOffsetXMm?: number | null,
  tsplOffsetYMm?: number | null,
): Promise<PrinterSettings> {
  const wire = await call<{
    serial_port: string;
    serial_baud: number;
    paper_width_dots: number;
    paper_width_mm: number;
    transport: string;
    queue_name: string;
    ticket_render_mode: string;
    tspl_probe_mode: string;
    tspl_gap_mm: number;
    tspl_density: number;
    tspl_speed: number;
    tspl_offset_x_mm: number;
    tspl_offset_y_mm: number;
  }>("printer_update_settings", {
    request: {
      session_token: sessionToken,
      serial_port: serialPort,
      serial_baud: serialBaud,
      paper_width_mm: paperWidthMm,
      transport,
      queue_name: queueName,
      // null/absent keeps the stored mode server-side (BRK-090 None-keeps).
      ticket_render_mode: ticketRenderMode ?? null,
      // TEMPORARY-DIAGNOSTIC: typed union — invalid spellings unconstructible.
      tspl_probe_mode: tsplProbeMode ?? null,
      tspl_gap_mm: tsplGapMm ?? null,
      tspl_density: tsplDensity ?? null,
      tspl_speed: tsplSpeed ?? null,
      tspl_offset_x_mm: tsplOffsetXMm ?? null,
      tspl_offset_y_mm: tsplOffsetYMm ?? null,
    },
  });
  return {
    serialPort: wire.serial_port,
    serialBaud: wire.serial_baud,
    paperWidthDots: wire.paper_width_dots,
    paperWidthMm: wire.paper_width_mm,
    transport: wire.transport === 'spooler' ? 'spooler' : 'serial',
    queueName: typeof wire.queue_name === 'string' ? wire.queue_name : '',
    receiptTransport: toRoleTransport(
      (wire as { receipt_transport?: unknown }).receipt_transport,
      'serial',
    ),
    receiptQueueName: toQueueName(
      (wire as { receipt_queue_name?: unknown }).receipt_queue_name,
    ),
    receiptQueueNameFallback: toQueueName(
      (wire as { receipt_queue_name_fallback?: unknown }).receipt_queue_name_fallback,
    ),
    labelTransport: toRoleTransport(
      (wire as { label_transport?: unknown }).label_transport,
      'spooler',
    ),
    labelQueueName: toQueueName(
      (wire as { label_queue_name?: unknown }).label_queue_name,
    ),
    labelQueueNameFallback: toQueueName(
      (wire as { label_queue_name_fallback?: unknown }).label_queue_name_fallback,
    ),
    ticketRenderMode: toTicketRenderMode(wire.ticket_render_mode),
    tsplProbeMode: toTsplProbeMode(wire.tspl_probe_mode),
    tsplGapMm: wire.tspl_gap_mm,
    tsplDensity: wire.tspl_density,
    tsplSpeed: wire.tspl_speed,
    tsplOffsetXMm: wire.tspl_offset_x_mm,
    tsplOffsetYMm: wire.tspl_offset_y_mm,
  };
}

/**
 * BRK-113 role patch: every field optional — `undefined` keeps the stored
 * value server-side; an empty fallback string clears it. Call ONLY when at
 * least one role field is addressed (the command layer skips otherwise).
 */
export interface PrinterRolePatch {
  receiptTransport?: PrinterTransport;
  receiptQueueName?: string;
  receiptQueueNameFallback?: string;
  labelTransport?: PrinterTransport;
  labelQueueName?: string;
  labelQueueNameFallback?: string;
}

/** Persist the dual-printer role routing (ADMIN only server-side). */
export async function ipcUpdatePrinterRoleSettings(
  sessionToken: string,
  patch: PrinterRolePatch,
): Promise<PrinterSettings> {
  const wire = await call<{
    serial_port: string;
    serial_baud: number;
    paper_width_dots: number;
    paper_width_mm: number;
    transport: string;
    queue_name: string;
    receipt_transport: string;
    receipt_queue_name: string;
    receipt_queue_name_fallback: string;
    label_transport: string;
    label_queue_name: string;
    label_queue_name_fallback: string;
    ticket_render_mode: string;
    tspl_probe_mode: string;
    tspl_gap_mm: number;
    tspl_density: number;
    tspl_speed: number;
    tspl_offset_x_mm: number;
    tspl_offset_y_mm: number;
  }>("printer_update_settings", {
    request: {
      session_token: sessionToken,
      // Role keys only — every other key stays untouched server-side.
      receipt_transport: patch.receiptTransport ?? null,
      receipt_queue_name: patch.receiptQueueName ?? null,
      receipt_queue_name_fallback: patch.receiptQueueNameFallback ?? null,
      label_transport: patch.labelTransport ?? null,
      label_queue_name: patch.labelQueueName ?? null,
      label_queue_name_fallback: patch.labelQueueNameFallback ?? null,
    },
  });
  return {
    serialPort: wire.serial_port,
    serialBaud: wire.serial_baud,
    paperWidthDots: wire.paper_width_dots,
    paperWidthMm: wire.paper_width_mm,
    transport: wire.transport === 'spooler' ? 'spooler' : 'serial',
    queueName: toQueueName(wire.queue_name),
    receiptTransport: toRoleTransport(wire.receipt_transport, 'serial'),
    receiptQueueName: toQueueName(wire.receipt_queue_name),
    receiptQueueNameFallback: toQueueName(wire.receipt_queue_name_fallback),
    labelTransport: toRoleTransport(wire.label_transport, 'spooler'),
    labelQueueName: toQueueName(wire.label_queue_name),
    labelQueueNameFallback: toQueueName(wire.label_queue_name_fallback),
    ticketRenderMode: toTicketRenderMode(wire.ticket_render_mode),
    tsplProbeMode: toTsplProbeMode(wire.tspl_probe_mode),
    tsplGapMm: wire.tspl_gap_mm,
    tsplDensity: wire.tspl_density,
    tsplSpeed: wire.tspl_speed,
    tsplOffsetXMm: wire.tspl_offset_x_mm,
    tsplOffsetYMm: wire.tspl_offset_y_mm,
  };
}

/** Fiscal stamp settings view (ADR-092: `fiscal.timbre_enabled`, default true). */
export interface FiscalSettings {
  timbreEnabled: boolean;
}

/** Read the fiscal stamp toggle (any authenticated session). */
export async function ipcGetFiscalSettings(sessionToken: string): Promise<FiscalSettings> {
  const wire = await call<{ timbre_enabled: boolean }>("fiscal_settings_get", {
    session_token: sessionToken,
  });
  return { timbreEnabled: wire.timbre_enabled };
}

/** Persist the fiscal stamp toggle (ADMIN only server-side). */
export async function ipcUpdateFiscalSettings(
  sessionToken: string,
  input: FiscalSettings,
): Promise<FiscalSettings> {
  const wire = await call<{ timbre_enabled: boolean }>("fiscal_settings_update", {
    request: { session_token: sessionToken, timbre_enabled: input.timbreEnabled },
  });
  return { timbreEnabled: wire.timbre_enabled };
}

/** Render + transmit a bilingual test page (ADMIN only server-side). */
export async function ipcTestPrint(sessionToken: string): Promise<PrintStatus> {
  const wire = await call<{
    job_id: string;
    status: string;
    attempt_count: string;
  }>("printer_test_print", { session_token: sessionToken });
  return {
    jobId: wire.job_id,
    status: wire.status,
    attemptCount: wire.attempt_count,
  };
}

// Store identity + print diagnostics bridge (TASK-BRK-056) -----------------
// Backend command mapping (sovereign contract):
// - store_identity_get / store_identity_save / printer_test_print —
//   exact backend names. Rust DTOs are canonical: snake_case wire fields,
//   device_kind wire values "physical" | "capture", device_outcome
//   "capture-only" | "sent" | classified device-error code.

/** Store identity: all string fields, empty when unset. */
export interface StoreIdentity {
  storeName: string;
  storeAddress: string;
  storePhone: string;
  nif: string;
  nis: string;
  rc: string;
  ai: string;
  ticketFooterMessage: string;
}

/** Diagnostics device kind (saved setting, selects the test-print path). */
export type DeviceKind = "physical" | "capture";

/** Test-print receipt language, fixed at call time. */
export type TestPrintLanguage = "ar" | "fr";

export interface StoreIdentityState {
  identity: StoreIdentity;
  deviceKind: DeviceKind;
}

/** One monospace preview line (shared layout blueprint, pre-raster). */
export interface PreviewLine {
  kind: string;
  centered: boolean;
  sizePx: number;
  text: string;
}

/** Structural verdict over the captured artifact. */
export interface DecodeSummary {
  bands: number;
  feedLines: number;
  unknownSpans: number;
}

export interface TestPrintResult {
  preview: PreviewLine[];
  decodeSummary: DecodeSummary;
  captureFileName: string;
  deviceOutcome: string;
}

interface StoreIdentityWire {
  store_name: string;
  store_address: string;
  store_phone: string;
  nif: string;
  nis: string;
  rc: string;
  ai: string;
  ticket_footer_message: string;
}

function toIdentity(wire: StoreIdentityWire): StoreIdentity {
  return {
    storeName: wire.store_name,
    storeAddress: wire.store_address,
    storePhone: wire.store_phone,
    nif: wire.nif,
    nis: wire.nis,
    rc: wire.rc,
    ai: wire.ai,
    ticketFooterMessage: wire.ticket_footer_message,
  };
}

function fromIdentity(identity: StoreIdentity): StoreIdentityWire {
  return {
    store_name: identity.storeName,
    store_address: identity.storeAddress,
    store_phone: identity.storePhone,
    nif: identity.nif,
    nis: identity.nis,
    rc: identity.rc,
    ai: identity.ai,
    ticket_footer_message: identity.ticketFooterMessage,
  };
}

/** Read identity + device kind (session-gated server-side). */
export async function getStoreIdentity(): Promise<StoreIdentityState> {
  const wire = await call<{
    identity: StoreIdentityWire;
    device_kind: string;
  }>("store_identity_get", {});
  return {
    identity: toIdentity(wire.identity),
    deviceKind: wire.device_kind === "capture" ? "capture" : "physical",
  };
}

/** Persist identity + device kind (SystemSettingsManage server-side). */
export async function saveStoreIdentity(
  identity: StoreIdentity,
  deviceKind: DeviceKind,
): Promise<StoreIdentityState> {
  const wire = await call<{
    identity: StoreIdentityWire;
    device_kind: string;
  }>("store_identity_save", {
    request: { identity: fromIdentity(identity), device_kind: deviceKind },
  });
  return {
    identity: toIdentity(wire.identity),
    deviceKind: wire.device_kind === "capture" ? "capture" : "physical",
  };
}

/** Read the active vertical profile + tier vocabulary (BRK-117, DSN-105 uses this). */
export async function getVerticalProfile(): Promise<VerticalProfileInfo> {
  const wire = await call<{
    profile: string;
    offered_tiers: string[];
    default_tier: string;
  }>("vertical_profile_get", {});
  return {
    profile: wire.profile,
    offeredTiers: wire.offered_tiers,
    defaultTier: wire.default_tier,
  };
}

/** Switch the vertical profile (SystemSettingsManage server-side). */
export async function setVerticalProfile(profile: string): Promise<VerticalProfileInfo> {
  const wire = await call<{
    profile: string;
    offered_tiers: string[];
    default_tier: string;
  }>("vertical_profile_set", { request: { profile } });
  return {
    profile: wire.profile,
    offeredTiers: wire.offered_tiers,
    defaultTier: wire.default_tier,
  };
}

/**
 * Synchronous diagnostics run (SystemSettingsManage server-side).
 * Kind is NOT a parameter: the backend uses the SAVED device_kind.
 */
export async function printingTestPrint(language: TestPrintLanguage): Promise<TestPrintResult> {
  const wire = await call<{
    preview: Array<{ kind: string; centered: boolean; size_px: number; text: string }>;
    decode_summary: { bands: number; feed_lines: number; unknown_spans: number };
    capture_file_name: string;
    device_outcome: string;
  }>("printer_test_print", { request: { language } });
  return {
    preview: wire.preview.map((line) => ({
      kind: line.kind,
      centered: line.centered,
      sizePx: line.size_px,
      text: line.text,
    })),
    decodeSummary: {
      bands: wire.decode_summary.bands,
      feedLines: wire.decode_summary.feed_lines,
      unknownSpans: wire.decode_summary.unknown_spans,
    },
    captureFileName: wire.capture_file_name,
    deviceOutcome: wire.device_outcome,
  };
}

/**
 * Fixed-content 40x20 mm TSPL self-test (TASK-PRT-01 — LIVE-2026-11-06).
 * Session only on the wire; content, media and probe mode come from
 * storage. Use this — never the ESC/POS ticket test — on TSPL printers.
 */
export interface TsplTestLabelResult {
  bytesSent: number;
}

export async function printingTestTsplLabel(): Promise<TsplTestLabelResult> {
  const wire = await call<{ bytes_sent: number }>("printer_test_tspl_label", {
    request: {},
  });
  return { bytesSent: wire.bytes_sent };
}

/**
 * Fixed-content 80 mm TSPL ticket self-test (TASK-PRT-03).
 * Session only on the wire; STORED media, one copy.
 */
export interface TsplTestTicketResult {
  bytesSent: number;
}

export async function printingTestTsplTicket(): Promise<TsplTestTicketResult> {
  const wire = await call<{ bytes_sent: number }>("printer_test_tspl_ticket", {
    request: {},
  });
  return { bytesSent: wire.bytes_sent };
}

/**
 * DOC-07 final 80 mm ticket ruler: full-width 1 mm ticks for measuring the
 * printer's true right dead zone. Session only on the wire; STORED media,
 * one copy.
 */
export interface TsplTicketRulerResult {
  bytesSent: number;
}

export async function printingTestTsplTicketRuler(): Promise<TsplTicketRulerResult> {
  const wire = await call<{ bytes_sent: number }>("printer_test_tspl_ticket_ruler", {
    request: {},
  });
  return { bytesSent: wire.bytes_sent };
}

/**
 * DOC-06 calibration pattern (40x20 ruler printout).
 * Session only on the wire; the pattern renders with the STORED media +
 * calibration, one copy.
 */
export interface TsplCalibrationResult {
  bytesSent: number;
}

export async function printingTestTsplCalibration(): Promise<TsplCalibrationResult> {
  const wire = await call<{ bytes_sent: number }>(
    "printer_test_tspl_calibration",
    {
      request: {},
    },
  );
  return { bytesSent: wire.bytes_sent };
}
