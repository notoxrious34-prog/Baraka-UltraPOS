/**
 * Contract tests for the category IPC wrappers (TASK-BRK-065).
 *
 * Pins the exact backend command names (`category_list`, `category_create`,
 * `category_update`, `category_seed_starter`), the wire shapes
 * field-for-field (snake_case; code immutable on update), and verbatim
 * error propagation.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  ipcCreateCategory,
  ipcListCategories,
  ipcSeedStarterCategories,
  ipcUpdateCategory,
} from "@/lib/ipc";
import { useSessionStore } from "@/stores/session-store";
import type { Category, SessionInfo } from "@/types/pos";

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
  };
  useSessionStore.setState({ session });
}

const stored: Category = {
  id: "11111111-1111-4111-8111-111111111111",
  code: "CAT-PLOMB",
  name_fr: "Plomberie & Sanitaire",
  name_ar: "الترصيص الصحي والغاز",
  parent_id: null,
  is_active: true,
};

describe("ipcListCategories", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("calls category_list with the bare token shape", async () => {
    primeSession("tok-cat-1");
    invokeMock.mockResolvedValueOnce([stored]);
    const rows = await ipcListCategories("tok-cat-1");
    expect(rows).toEqual([stored]);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("category_list", {
      session_token: "tok-cat-1",
      include_inactive: false,
    });
  });

  it("forwards includeInactive when requested", async () => {
    primeSession("tok-cat-1");
    invokeMock.mockResolvedValueOnce([stored]);
    await ipcListCategories("tok-cat-1", true);
    expect(invokeMock).toHaveBeenCalledWith("category_list", {
      session_token: "tok-cat-1",
      include_inactive: true,
    });
  });

  it("rejects verbatim when the backend refuses", async () => {
    primeSession("tok-cat-1");
    invokeMock.mockRejectedValueOnce(new Error("UNAUTHORIZED"));
    await expect(ipcListCategories("tok-cat-1")).rejects.toThrow("UNAUTHORIZED");
  });
});

describe("ipcCreateCategory", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("calls category_create with the exact request payload", async () => {
    primeSession("tok-cat-2");
    invokeMock.mockResolvedValueOnce(stored);
    const created = await ipcCreateCategory("tok-cat-2", {
      code: "CAT-PLOMB",
      name_fr: "Plomberie & Sanitaire",
      name_ar: "الترصيص الصحي والغاز",
      parent_id: null,
    });
    expect(created).toEqual(stored);
    expect(invokeMock).toHaveBeenCalledWith("category_create", {
      request: {
        session_token: "tok-cat-2",
        code: "CAT-PLOMB",
        name_fr: "Plomberie & Sanitaire",
        name_ar: "الترصيص الصحي والغاز",
        parent_id: null,
      },
      session_token: "tok-cat-2",
    });
  });

  it("surfaces CATEGORY_CODE_DUPLICATE verbatim", async () => {
    primeSession("tok-cat-2");
    invokeMock.mockRejectedValueOnce(new Error("CATEGORY_CODE_DUPLICATE"));
    await expect(
      ipcCreateCategory("tok-cat-2", {
        code: "CAT-PLOMB",
        name_fr: "Plomberie & Sanitaire",
        name_ar: "الترصيص الصحي والغاز",
      }),
    ).rejects.toThrow("CATEGORY_CODE_DUPLICATE");
  });
});

describe("ipcUpdateCategory", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("calls category_update without any code field (immutability by construction)", async () => {
    primeSession("tok-cat-3");
    invokeMock.mockResolvedValueOnce({ ...stored, is_active: false });
    const updated = await ipcUpdateCategory("tok-cat-3", {
      category_id: "11111111-1111-4111-8111-111111111111",
      name_fr: "Plomberie MAJ",
      name_ar: "الترصيص الصحي والغاز",
      parent_id: null,
      is_active: false,
    });
    expect(updated.is_active).toBe(false);
    expect(invokeMock).toHaveBeenCalledWith("category_update", {
      request: {
        session_token: "tok-cat-3",
        category_id: "11111111-1111-4111-8111-111111111111",
        name_fr: "Plomberie MAJ",
        name_ar: "الترصيص الصحي والغاز",
        parent_id: null,
        is_active: false,
      },
      session_token: "tok-cat-3",
    });
    const args = invokeMock.mock.calls[0]?.[1] as {
      request: Record<string, unknown>;
    };
    expect("code" in args.request).toBe(false);
  });
});

describe("ipcSeedStarterCategories", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("calls category_seed_starter and resolves the seven departments", async () => {
    primeSession("tok-cat-4");
    const seven: Category[] = [
      "CAT-PLOMB",
      "CAT-ELEC",
      "CAT-VISS",
      "CAT-OUTIL",
      "CAT-PEINT",
      "CAT-BATIM",
      "CAT-JARDIN",
    ].map((code, index) => ({ ...stored, id: `id-${index}`, code }));
    invokeMock.mockResolvedValueOnce(seven);
    const rows = await ipcSeedStarterCategories("tok-cat-4");
    expect(rows).toHaveLength(7);
    expect(rows.map((row) => row.code)).toEqual([
      "CAT-PLOMB",
      "CAT-ELEC",
      "CAT-VISS",
      "CAT-OUTIL",
      "CAT-PEINT",
      "CAT-BATIM",
      "CAT-JARDIN",
    ]);
    expect(invokeMock).toHaveBeenCalledWith("category_seed_starter", {
      session_token: "tok-cat-4",
    });
  });
});
