/**
 * Contract tests for the document-query IPC wrappers (TASK-BRK-042).
 *
 * Pins the documents_search command shape (filters + clamped limit, newest
 * first) and the sales_get flat-arg convention behind getDocumentDetails,
 * plus code-preserving error propagation — so a drift between these
 * wrappers and the Rust DTOs fails here, not in the conversion flow.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  getDocumentDetails,
  searchDocuments,
  type DocumentSearchWireRequest,
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

function primeSession(token: string): void {
  const session: SessionInfo = {
    token,
    userId: "55555555-5555-5555-5555-555555555555",
    username: "gerant1",
    role: "GERANT",
    expiresAt: "2026-01-01T00:00:00Z",
    permissions: [],
  };
  useSessionStore.setState({ session });
}

const searchRequest: DocumentSearchWireRequest = {
  session_token: "tok-search-1",
  document_type: "DEVIS",
  status: "FINALIZED",
  query: "000001",
  limit: 20,
};

const searchResponse = {
  documents: [
    {
      id: "22222222-2222-4222-8222-222222222222",
      document_number: "DEV-2026-000001",
      document_type: "DEVIS",
      status: "FINALIZED",
      party_id: null,
      party_name: null,
      total_ttc_centimes: "40000",
      items_count: 2,
      created_at: "2026-01-01T00:00:00Z",
    },
  ],
};

const detailsResponse = {
  document: {
    document_id: "22222222-2222-4222-8222-222222222222",
    number: "DEV-2026-000001",
    doc_type: "DEVIS",
    status: "FINALIZED",
    warehouse_id: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
    subtotal_ht_centimes: "40000",
    tva_total_centimes: "0",
    total_ttc_centimes: "40000",
    paid_amount_centimes: "0",
    change_amount_centimes: "0",
    payment_status: "UNPAID",
  },
  items: [],
  payments: [],
  print_status: "NONE",
};

describe("searchDocuments", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("calls documents_search with filters and injected session token", async () => {
    primeSession("tok-search-1");
    invokeMock.mockResolvedValueOnce(searchResponse);
    const result = await searchDocuments(searchRequest);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("documents_search", {
      request: {
        session_token: "tok-search-1",
        document_type: "DEVIS",
        status: "FINALIZED",
        query: "000001",
        limit: 20,
      },
      session_token: "tok-search-1",
    });
    expect(result).toEqual(searchResponse);
  });

  it("preserves canonical money strings verbatim", async () => {
    primeSession("tok-search-1");
    invokeMock.mockResolvedValueOnce(searchResponse);
    const result = await searchDocuments({ ...searchRequest, query: null });
    expect(result.documents[0]?.total_ttc_centimes).toBe("40000");
    expect(typeof result.documents[0]?.total_ttc_centimes).toBe("string");
  });

  it("propagates backend error codes without alteration", async () => {
    primeSession("tok-search-1");
    for (const code of ["UNAUTHORIZED", "INVALID_INPUT"]) {
      invokeMock.mockRejectedValueOnce(code);
      await expect(searchDocuments(searchRequest)).rejects.toBe(code);
    }
    expect(invokeMock).toHaveBeenCalledTimes(2);
  });

  it("rejects with IPC_NO_ACTIVE_SESSION before invoking when no session is primed", async () => {
    useSessionStore.setState({ session: null });
    await expect(searchDocuments(searchRequest)).rejects.toThrow("IPC_NO_ACTIVE_SESSION");
    expect(invokeMock).not.toHaveBeenCalled();
  });
});

describe("getDocumentDetails", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("calls sales_get with flat scalar args (no nested request object)", async () => {
    primeSession("tok-search-2");
    invokeMock.mockResolvedValueOnce(detailsResponse);
    const result = await getDocumentDetails("22222222-2222-4222-8222-222222222222");
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("sales_get", {
      session_token: "tok-search-2",
      document_id: "22222222-2222-4222-8222-222222222222",
    });
    expect(result).toEqual(detailsResponse);
  });

  it("propagates DOCUMENT_NOT_FOUND without alteration", async () => {
    primeSession("tok-search-2");
    invokeMock.mockRejectedValueOnce("DOCUMENT_NOT_FOUND");
    await expect(getDocumentDetails("00000000-0000-4000-8000-000000000000")).rejects.toBe(
      "DOCUMENT_NOT_FOUND",
    );
  });
});
