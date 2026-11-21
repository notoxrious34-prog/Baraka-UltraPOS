/**
 * Échange Express section (BRK-125).
 *
 * Staged inside the return workstation: the return quantities picked above
 * are reused verbatim, alternates are added through catalog search with a
 * per-row unit selector and micro quantity, the net is computed server-side
 * (last-price memory — the UI never prices), and an optional Gérant PIN
 * unlocks below-cost alternate legs for this exchange only. All money
 * renders via formatMoney and computes via BigInt — zero floats.
 */

import { useEffect, useMemo, useRef, useState, type ReactElement } from "react";
import { useReturnModalStore, parseReturnQtyInput } from "@/stores/return-modal-store";
import { t, translateBackendError } from "@/lib/i18n";
import { createDebouncedSearchRunner } from "@/lib/debounced-search";
import { formatMoney, formatQty } from "@/lib/money";
import { ipcReprint } from "@/lib/ipc";
import { useSessionStore } from "@/stores/session-store";
import type { Locale } from "@/types/pos";
import type { CatalogProduct } from "@/types/pos";

function fmtCents(cents: bigint, locale: Locale): string {
  try {
    return formatMoney(cents.toString(), locale);
  } catch {
    return "…";
  }
}

function fmtQty(micro: bigint): string {
  try {
    return formatQty(micro.toString());
  } catch {
    return "…";
  }
}

function tryBigInt(value: string): bigint {
  try {
    return BigInt(value);
  } catch {
    return 0n;
  }
}

interface RowDraft {
  unitId: string;
  qty: string;
}

function rowKey(productId: string): string {
  return productId;
}

export function ExchangeSection({ locale }: { locale: Locale }): ReactElement {
  const s = useReturnModalStore();
  const session = useSessionStore((state) => state.session);
  const [printing, setPrinting] = useState(false);
  const [drafts, setDrafts] = useState<Record<string, RowDraft>>({});

  const searchRef = useRef<ReturnType<typeof createDebouncedSearchRunner> | null>(null);
  if (searchRef.current === null) {
    searchRef.current = createDebouncedSearchRunner({
      delayMs: 200,
      shouldSearch: (firedQuery) => firedQuery.trim().length >= 2,
      onSearch: (firedQuery) => {
        void s.searchAlternates(firedQuery);
      },
    });
  }
  useEffect(() => {
    searchRef.current?.search(s.altSearchQuery);
  }, [s.altSearchQuery]);
  useEffect(
    () => () => {
      searchRef.current?.dispose();
    },
    [],
  );

  const canSubmit =
    !s.isExchanging &&
    !s.isSubmitting &&
    s.context !== null &&
    s.exchangeAlternates.length > 0 &&
    Object.values(s.quantities).some((q) => {
      try {
        return BigInt(q) > 0n;
      } catch {
        return false;
      }
    });

  const netPreview = useMemo(() => {
    if (s.exchangeResult === null) {
      return null;
    }
    return tryBigInt(s.exchangeResult.net_centimes);
  }, [s.exchangeResult]);

  const printSale = (): void => {
    if (session === null || s.exchangeResult === null || printing) {
      return;
    }
    setPrinting(true);
    ipcReprint(session.token, s.exchangeResult.sale_document_id)
      .catch(() => {
        // Outbox-tracked server-side; the exchange is committed.
      })
      .finally(() => {
        setPrinting(false);
      });
  };

  return (
    <div className="flex flex-col gap-2 rounded-lg border-2 border-dashed border-primary-400 px-3 py-2 dark:border-primary-700">
      <p className="text-sm font-black text-primary-700 dark:text-primary-300">
        {t(locale, "exchange.sectionTitle")}
      </p>
      <p className="text-[11px] font-bold text-slate-500">
        {t(locale, "exchange.serverPriceNote")}
      </p>

      {s.exchangeResult !== null ? (
        <div className="flex flex-col gap-2">
          <p className="rounded-lg bg-emerald-50 px-3 py-3 text-center text-base font-black text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
            {t(locale, "exchange.success")
              .replace("{sale}", s.exchangeResult.sale_document_number)
              .replace("{avoir}", s.exchangeResult.return_document_number)}
          </p>
          <p className="text-center font-mono text-2xl font-black tabular-nums">
            {t(locale, "exchange.netLabel")}:{" "}
            {netPreview !== null ? fmtCents(netPreview, locale) : "…"}
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={printSale}
              disabled={printing}
              className="h-11 flex-1 rounded-lg bg-primary-600 text-sm font-black text-white disabled:opacity-50"
            >
              {printing ? t(locale, "returns.printing") : t(locale, "exchange.printSale")}
            </button>
            <button
              type="button"
              onClick={() => {
                useReturnModalStore.setState({
                  exchangeResult: null,
                  exchangeAlternates: [],
                });
              }}
              className="h-11 flex-1 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-100"
            >
              {t(locale, "exchange.newExchange")}
            </button>
          </div>
        </div>
      ) : (
        <>
          <input
            value={s.altSearchQuery}
            onChange={(event) => {
              s.setAltSearchQuery(event.target.value);
            }}
            placeholder={t(locale, "exchange.searchPlaceholder")}
            autoComplete="off"
            dir="auto"
            disabled={s.isExchanging || s.isSearchingAlts}
            className="h-10 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
          />
          {s.isSearchingAlts && (
            <p className="text-center text-xs font-bold text-slate-500">
              {t(locale, "returns.searching")}
            </p>
          )}
          {s.altSearchResults.map((product: CatalogProduct) => {
            const draft = drafts[rowKey(product.product_id)] ?? {
              unitId: product.units.find((u) => u.is_default_selling)?.unit_id ?? product.units[0]?.unit_id ?? "",
              qty: "1",
            };
            return (
              <div
                key={product.product_id}
                className="flex flex-col gap-1 rounded-md bg-slate-100 px-2 py-2 dark:bg-slate-800"
              >
                <span className="truncate text-sm font-extrabold">
                  {product.name_fr !== "" ? product.name_fr : product.name_ar}
                </span>
                <div className="flex gap-2">
                  <select
                    value={draft.unitId}
                    onChange={(event) => {
                      const unitId = event.target.value;
                      setDrafts((prev) => ({
                        ...prev,
                        [rowKey(product.product_id)]: { unitId, qty: prev[rowKey(product.product_id)]?.qty ?? "1" },
                      }));
                    }}
                    disabled={s.isExchanging}
                    className="h-9 min-w-0 flex-1 rounded-md border border-slate-300 bg-white px-1 text-xs font-bold outline-none disabled:opacity-40 dark:border-slate-700 dark:bg-slate-900"
                  >
                    {product.units.map((unit) => (
                      <option key={unit.unit_id} value={unit.unit_id}>
                        {unit.unit_name_fr !== "" ? unit.unit_name_fr : unit.unit_name_ar}
                      </option>
                    ))}
                  </select>
                  <input
                    value={draft.qty}
                    onChange={(event) => {
                      const qty = event.target.value;
                      setDrafts((prev) => ({
                        ...prev,
                        [rowKey(product.product_id)]: { unitId: draft.unitId, qty },
                      }));
                    }}
                    inputMode="decimal"
                    autoComplete="off"
                    dir="ltr"
                    disabled={s.isExchanging}
                    className="h-9 w-20 rounded-md border border-slate-300 bg-white px-2 font-mono text-sm font-black outline-none focus:border-primary-600 disabled:opacity-40 dark:border-slate-700 dark:bg-slate-900"
                  />
                  <button
                    type="button"
                    disabled={s.isExchanging}
                    onClick={() => {
                      try {
                        s.addAlternate(product, draft.unitId, parseReturnQtyInput(draft.qty).toString());
                      } catch {
                        // Invalid quantities are ignored; the store guards too.
                      }
                      setDrafts((prev) => {
                        const next = { ...prev };
                        delete next[rowKey(product.product_id)];
                        return next;
                      });
                    }}
                    className="h-9 shrink-0 rounded-md bg-primary-600 px-3 text-xs font-black text-white disabled:opacity-50"
                  >
                    {t(locale, "exchange.add")}
                  </button>
                </div>
              </div>
            );
          })}

          {s.exchangeAlternates.length > 0 && (
            <div className="flex flex-col gap-1">
              {s.exchangeAlternates.map((alt, index) => (
                <div
                  key={`${alt.product_id}-${alt.unit_id}-${index}`}
                  className="flex items-center justify-between gap-2 rounded-md bg-primary-50 px-2 py-1.5 text-sm dark:bg-primary-900/20"
                >
                  <span className="min-w-0 truncate font-bold">
                    {alt.product_name} · {alt.unit_name} ·{" "}
                    <bdi dir="ltr" className="font-mono">
                      {fmtQty(tryBigInt(alt.quantity_micro))}
                    </bdi>
                  </span>
                  <button
                    type="button"
                    disabled={s.isExchanging}
                    onClick={() => {
                      s.removeAlternate(index);
                    }}
                    className="shrink-0 rounded-md bg-red-100 px-2 py-1 text-xs font-black text-red-700 disabled:opacity-50 dark:bg-red-900/40 dark:text-red-200"
                  >
                    {t(locale, "exchange.remove")}
                  </button>
                </div>
              ))}
            </div>
          )}

          <div className="grid grid-cols-2 gap-2">
            <input
              value={s.managerUsername}
              onChange={(event) => {
                s.setManagerUsername(event.target.value);
              }}
              placeholder={t(locale, "exchange.managerUser")}
              autoComplete="off"
              disabled={s.isExchanging}
              className="h-10 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
            />
            <input
              value={s.managerPin}
              onChange={(event) => {
                s.setManagerPin(event.target.value);
              }}
              placeholder={t(locale, "exchange.managerPin")}
              type="password"
              inputMode="numeric"
              autoComplete="off"
              disabled={s.isExchanging}
              className="h-10 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
            />
          </div>

          {s.exchangeError !== null && (
            <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
              {translateBackendError(s.exchangeError, locale)}
            </p>
          )}

          <button
            type="button"
            onClick={() => {
              void s.submitExchange();
            }}
            disabled={!canSubmit}
            className="h-11 w-full rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
          >
            {s.isExchanging
              ? t(locale, "exchange.submitting")
              : t(locale, "exchange.confirm")}
          </button>
        </>
      )}
    </div>
  );
}
