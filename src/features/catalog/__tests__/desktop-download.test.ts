/**
 * Desktop download helper tests (TASK-REL-055).
 *
 * The rc.6 field defect: WebView2 dispatches blob downloads asynchronously,
 * so the old synchronous `URL.revokeObjectURL` voided the URL before the
 * pickup and both CSV buttons silently did nothing. These tests install a
 * minimal deterministic DOM/URL double and pin the fixed contract:
 * createObjectURL is called, the anchor click is dispatched, revocation is
 * DEFERRED (never synchronous), and the outcome echoes the filename.
 */

import { afterEach, describe, expect, it, vi } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import {
  CSV_MIME_TYPE,
  DOWNLOAD_URL_LIFETIME_MS,
  downloadTextFile,
} from "@/features/catalog/desktop-download";

interface FakeAnchor {
  href: string;
  download: string;
  dispatched: number;
  removed: boolean;
}

function installDomDouble() {
  const revoked: string[] = [];
  const created: string[] = [];
  const timers: Array<{ fn: () => void; ms: number }> = [];
  const anchor: FakeAnchor = {
    href: "",
    download: "",
    dispatched: 0,
    removed: false,
  };
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.Blob = class {
    constructor(
      public parts: string[],
      public options: { type: string },
    ) {}
  };
  globals.URL = {
    createObjectURL: (_blob: { parts: string[] }) => {
      const url = `blob:baraka/${created.length}`;
      created.push(url);
      return url;
    },
    revokeObjectURL: (url: string) => {
      revoked.push(url);
    },
  };
  globals.document = {
    body: {
      appendChild: () => undefined,
    },
    createElement: () => ({
      href: "",
      download: "",
      dispatchEvent: () => {
        anchor.dispatched += 1;
      },
      remove: () => {
        anchor.removed = true;
      },
    }),
    querySelectorAll: () => [],
  };
  globals.MouseEvent = class {
    constructor(
      public type: string,
      public init: Record<string, unknown>,
    ) {}
  };
  globals.setTimeout = ((fn: () => void, ms: number) => {
    timers.push({ fn, ms });
    return timers.length;
  }) as unknown as typeof setTimeout;
  return { revoked, created, timers, anchor };
}

const original = {
  Blob: (globalThis as unknown as Record<string, unknown>).Blob,
  URL: (globalThis as unknown as Record<string, unknown>).URL,
  document: (globalThis as unknown as Record<string, unknown>).document,
  MouseEvent: (globalThis as unknown as Record<string, unknown>).MouseEvent,
  setTimeout: (globalThis as unknown as Record<string, unknown>).setTimeout,
};

afterEach(() => {
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.Blob = original.Blob;
  globals.URL = original.URL;
  globals.document = original.document;
  globals.MouseEvent = original.MouseEvent;
  globals.setTimeout = original.setTimeout;
  vi.restoreAllMocks();
});

describe("downloadTextFile (WebView2-safe path)", () => {
  it("is a guarded no-op without a DOM", () => {
    const needsDom =
      typeof (globalThis as unknown as Record<string, unknown>).document ===
      "undefined";
    if (!needsDom) {
      return;
    }
    const outcome = downloadTextFile("a;b", "x.csv");
    expect(outcome.ok).toBe(false);
    expect(outcome.reason).toBe("no-dom");
  });

  it("dispatches the click and defers revocation past the async pickup", () => {
    const dom = installDomDouble();
    const outcome = downloadTextFile("sku;qty\r\nA;1\r\n", "catalogue.csv");

    expect(outcome).toEqual({
      ok: true,
      filename: "catalogue.csv",
      reason: "started",
    });
    expect(dom.created).toHaveLength(1);
    // The regression: nothing may be revoked synchronously.
    expect(dom.revoked).toEqual([]);
    expect(dom.anchor.dispatched).toBe(1);
    expect(dom.anchor.removed).toBe(true);
    // Revocation is scheduled, with the documented lifetime.
    expect(dom.timers).toHaveLength(1);
    expect(dom.timers[0]?.ms).toBe(DOWNLOAD_URL_LIFETIME_MS);
    dom.timers[0]?.fn();
    expect(dom.revoked).toEqual(dom.created);
  });

  it("uses the UTF-8 CSV mime type by default and honours an override", () => {
    installDomDouble();
    const blobSpy: Array<{ parts: string[]; options: { type: string } }> = [];
    const globals = globalThis as unknown as Record<string, unknown>;
    globals.Blob = class {
      constructor(
        public parts: string[],
        public options: { type: string },
      ) {
        blobSpy.push({ parts, options });
      }
    };
    downloadTextFile("body", "a.csv");
    downloadTextFile("body", "b.txt", "text/plain;charset=utf-8");
    expect(blobSpy[0]?.options.type).toBe(CSV_MIME_TYPE);
    expect(blobSpy[1]?.options.type).toBe("text/plain;charset=utf-8");
  });

  it("keeps the deferred-revoke contract pinned in source", () => {
    const src = readFileSync(
      join(process.cwd(), "src", "features", "catalog", "desktop-download.ts"),
      "utf8",
    ) as string;
    expect(src).toContain("setTimeout(() => {");
    expect(src).toContain("revokeObjectURL(url)");
    expect(src).toContain("DOWNLOAD_URL_LIFETIME_MS");
    // No synchronous revoke may return to this file.
    expect(src).not.toMatch(/finally\s*\{[\s\S]*revokeObjectURL/);
  });
});

declare const process: { cwd(): string };
