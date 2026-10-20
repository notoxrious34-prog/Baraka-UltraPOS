/**
 * Catalog search: 150ms debounced backend search plus a barcode wedge
 * listener. Scanners act as keyboards: rapid character bursts (<30ms
 * mean interval) terminated by Enter resolve immediately to a product.
 */

import { useEffect, useRef, useState } from "react";
import type { CatalogProduct } from "@/types/pos";
import { ipcSearchCatalog } from "@/lib/ipc";
import { useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";

const DEBOUNCE_MS = 150;
const WEDGE_INTERVAL_MS = 30;
const MIN_SCAN_LENGTH = 4;

interface WedgeKey {
  char: string;
  at: number;
}

export interface CatalogSearch {
  results: CatalogProduct[];
  loading: boolean;
  error: string | null;
  scanFlash: boolean;
}

export function useCatalogSearch(): CatalogSearch {
  const searchQuery = usePosUiStore((state) => state.searchQuery);
  const [results, setResults] = useState<CatalogProduct[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [scanFlash, setScanFlash] = useState(false);
  const wedge = useRef<WedgeKey[]>([]);
  const flashTimer = useRef<number | null>(null);

  // Debounced backend search (blank queries fetch default discovery rows).
  useEffect(() => {
    const query = searchQuery.trim();
    setLoading(true);
    const timer = window.setTimeout(() => {
      const session = useSessionStore.getState().session;
      if (session === null) {
        setLoading(false);
        setError("UNAUTHORIZED");
        return;
      }
      // Discovery fetches the first 100 active rows; inactive products are
      // excluded server-side (includeInactive=false) so disabled items never
      // render as purchasable POS cards.
      ipcSearchCatalog(session.token, query, 100, false)
        .then((products) => {
          setResults(products);
          setError(null);
        })
        .catch((failure: unknown) => {
          setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
          setResults([]);
        })
        .finally(() => {
          setLoading(false);
        });
    }, DEBOUNCE_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [searchQuery]);

  // Barcode wedge: rapid burst + Enter resolves a product instantly.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      const ui = usePosUiStore.getState();
      if (ui.activeModal !== "none") {
        wedge.current = [];
        return;
      }
      const target = event.target;
      const inBody = target === document.body || target instanceof HTMLDivElement;
      if (event.key === "Enter") {
        const burst = wedge.current;
        wedge.current = [];
        // When focus already sits in the search input (the steady state after
        // the DSN-040 wedge redirect), the input's own Enter handler on
        // #pos-search-input is the single resolver — resolving here too would
        // double-add. The legacy burst path is kept for body-focus scans.
        if (burst.length < MIN_SCAN_LENGTH || !inBody) {
          return;
        }
        const first = burst[0];
        const last = burst[burst.length - 1];
        if (first === undefined || last === undefined) {
          return;
        }
        const span = last.at - first.at;
        const mean = span / burst.length;
        if (mean >= WEDGE_INTERVAL_MS) {
          return;
        }
        const code = burst.map((key) => key.char).join("");
        event.preventDefault();
        const session = useSessionStore.getState().session;
        if (session === null) {
          return;
        }
        setScanFlash(true);
        if (flashTimer.current !== null) {
          window.clearTimeout(flashTimer.current);
        }
        flashTimer.current = window.setTimeout(() => {
          setScanFlash(false);
        }, 600);
        ipcSearchCatalog(session.token, code, 5)
          .then((products) => {
            const exact = products.find((product) =>
              product.units.some((unit) => unit.barcode === code),
            );
            const pick = exact ?? products[0];
            if (pick !== undefined) {
              const matchedUnitIndex = pick.units.findIndex(
                (unit) => unit.barcode === code,
              );
              useCartStore
                .getState()
                .addProduct(pick, matchedUnitIndex >= 0 ? matchedUnitIndex : undefined);
              usePosUiStore.getState().setSearchQuery("");
            }
          })
          .catch(() => {
            // Scan misses stay silent; the typed query path remains visible.
          });
        return;
      }
      if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
        const now = performance.now();
        const kept = wedge.current.slice(-32);
        kept.push({ char: event.key, at: now });
        wedge.current = kept;
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      if (flashTimer.current !== null) {
        window.clearTimeout(flashTimer.current);
      }
    };
  }, []);

  return { results, loading, error, scanFlash };
}
