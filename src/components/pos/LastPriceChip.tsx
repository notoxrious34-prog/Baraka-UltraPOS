/**
 * Last-price memory chip (DSN-106): read-only recall of the transacted unit
 * price for this customer × product × unit (BRK-111). Pure display — the
 * engine never auto-applies it; the cashier decides. BigInt-only math,
 * silent collapse on guest/loading/failure/unknown.
 */
import { useEffect, useState, type ReactElement } from "react";
import { ipcGetLastPrice } from "@/lib/ipc";
import { formatMoney } from "@/lib/money";
import { t } from "@/lib/i18n";
import { useSessionStore } from "@/stores/session-store";
import type { LastPriceMemory, Locale } from "@/types/pos";

interface LastPriceChipProps {
  locale: Locale;
  /** Named-customer id; `null` (guest) renders nothing. */
  customerId: string | null;
  productId: string;
  unitId: string;
}

/** Effective transacted price (`price - discount`), garbage-safe. */
export function effectiveLastPrice(cell: LastPriceMemory): string | null {
  try {
    const net = BigInt(cell.priceCentimes) - BigInt(cell.discountPerUnitCentimes);
    return net < 0n ? null : net.toString();
  } catch {
    return null;
  }
}

export function LastPriceChip(props: LastPriceChipProps): ReactElement | null {
  const { locale, customerId, productId, unitId } = props;
  const token = useSessionStore((state) => state.session?.token ?? null);
  const [cell, setCell] = useState<LastPriceMemory | null>(null);

  useEffect(() => {
    if (token === null || customerId === null) {
      setCell(null);
      return;
    }
    let cancelled = false;
    // No module cache: a finalized sale changes the memory, and the cart
    // clears after checkout — per-mount fetch is always fresh here.
    ipcGetLastPrice(token, customerId, productId, unitId)
      .then((memory) => {
        if (!cancelled) {
          setCell(memory);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setCell(null);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token, customerId, productId, unitId]);

  if (cell === null) {
    return null;
  }
  const net = effectiveLastPrice(cell);
  if (net === null) {
    return null;
  }
  let label: string;
  try {
    label = formatMoney(net, locale);
  } catch {
    return null;
  }
  return (
    <span
      data-testid="cart-last-price"
      title={cell.soldAt}
      className="mt-0.5 inline-block truncate rounded bg-sky-100 px-1.5 py-0.5 text-[10px] font-bold text-sky-800 dark:bg-sky-900/40 dark:text-sky-300"
    >
      {t(locale, "cart.lastPrice")}: {label}
    </span>
  );
}
