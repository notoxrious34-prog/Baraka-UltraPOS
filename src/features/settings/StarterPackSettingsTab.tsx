/**
 * Starter-pack settings tab (TASK-DSN-053): pack lifecycle cockpit embedded
 * as an executive Settings card. Loads depots + catalog rows for install
 * detection, then mounts `StarterPackBanner`. Settings → Données → Pack.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t } from "@/lib/i18n";
import type { Locale } from "@/types/pos";
import type { CatalogProduct } from "@/types/pos";
import {
  ipcSearchCatalog,
  listWarehouses,
  type WarehouseItem,
} from "@/lib/ipc";
import { StarterPackBanner } from "@/features/catalog/StarterPackBanner";

interface StarterPackSettingsTabProps {
  locale: Locale;
  sessionToken: string;
}

const DETECTION_LIMIT = 1000;

export function StarterPackSettingsTab({
  locale,
  sessionToken,
}: StarterPackSettingsTabProps): ReactElement {
  const [warehouses, setWarehouses] = useState<WarehouseItem[]>([]);
  const [products, setProducts] = useState<CatalogProduct[]>([]);
  const [loading, setLoading] = useState(true);
  const [tick, setTick] = useState(0);

  useEffect(() => {
    let cancelled = false;
    const alive = (): boolean => !cancelled;
    setLoading(true);
    void (async () => {
      try {
        const [deps, rows] = await Promise.all([
          listWarehouses(sessionToken),
          ipcSearchCatalog(sessionToken, "", DETECTION_LIMIT),
        ]);
        if (alive()) {
          setWarehouses(deps);
          setProducts(rows);
        }
      } catch {
        if (alive()) {
          setWarehouses([]);
          setProducts([]);
        }
      } finally {
        if (alive()) {
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [sessionToken, tick]);

  if (loading) {
    return <p className="p-4 text-sm text-slate-500">{t(locale, "settings.loading")}</p>;
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
      <StarterPackBanner
        locale={locale}
        token={sessionToken}
        warehouses={warehouses}
        products={products}
        onChanged={() => {
          setTick((current) => current + 1);
        }}
      />
    </div>
  );
}
