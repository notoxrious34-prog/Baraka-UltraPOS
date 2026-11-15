/**
 * Quantity-break ladder loader (BRK-121).
 * Ensures every visible cart line carries its unit's ladder so
 * `lineUnitPriceCentimes` previews exactly what the engine commits.
 * Missing (`undefined`) triggers one fetch; loaded (`[]` = no ladder) and
 * failed (cached `[]`) never refetch until the line structure changes.
 */
import { useEffect } from "react";
import { ipcListPriceBreaks } from "@/lib/ipc";
import { useCartStore } from "@/stores/cart-store";
import { useSessionStore } from "@/stores/session-store";
import type { CartLine } from "@/types/pos";

export function useEnsurePriceBreaks(lines: CartLine[]): void {
  const token = useSessionStore((state) => state.session?.token ?? null);
  const setLineBreaks = useCartStore((state) => state.setLineBreaks);

  useEffect(() => {
    if (token === null) {
      return;
    }
    let cancelled = false;
    for (const line of lines) {
      const unit = line.units[line.unitIndex];
      if (unit === undefined || line.breaksByUnit[unit.unitId] !== undefined) {
        continue;
      }
      const { key, productId } = line;
      const { unitId } = unit;
      ipcListPriceBreaks(token, productId, unitId)
        .then((ladder) => {
          if (!cancelled) {
            setLineBreaks(
              key,
              unitId,
              ladder.map((rung) => ({
                minQtyMicro: rung.min_qty_micro,
                priceCentimes: rung.price_centimes,
              })),
            );
          }
        })
        .catch(() => {
          // Cache empty: never spam a struggling backend; a structural
          // change (new line, unit switch) refetches naturally.
          if (!cancelled) {
            setLineBreaks(key, unitId, []);
          }
        });
    }
    return () => {
      cancelled = true;
    };
  }, [token, lines, setLineBreaks]);
}
