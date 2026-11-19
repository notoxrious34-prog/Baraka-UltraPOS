/**
 * Priceless warehouse prep-sheet modal (BRK-126, `Bon de Préparation`).
 *
 * Explodes the live cart into whole-rung Colisage shares for the picker.
 * The sheet carries NO price — screen and paper show quantities only.
 * Print packs the off-screen 576-dot checklist through
 * `print_raw_bitmap` (WYSIWYG pixels, BRK-101).
 */

import { useEffect, useRef, useState, type ReactElement } from "react";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import { ipcBuildPrepSheet, type PrepSheetLineResult } from "@/lib/ipc";
import { ipcPrintRawBitmap } from "@/lib/ipc-printing";
import { rasterizeElement } from "@/lib/bitmap-capture";
import { useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { t } from "@/lib/i18n";

/** 72 mm checklist width at 8 dots/mm (byte-aligned for the packer). */
const PRINT_WIDTH_DOTS = 576;

export function PrepSheetModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const session = useSessionStore((state) => state.session);
  const lines = useCartStore((state) => state.lines);
  const [sheet, setSheet] = useState<PrepSheetLineResult[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [printing, setPrinting] = useState(false);
  const [printed, setPrinted] = useState<number | null>(null);
  const printRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (session === null) {
      setError("NO_SESSION");
      return;
    }
    let cancelled = false;
    ipcBuildPrepSheet(
      session.token,
      lines.map((line) => ({
        product_id: line.productId,
        unit_id: line.units[line.unitIndex]?.unitId ?? "",
        quantity_micro: line.quantityMicro,
      })),
    )
      .then((result) => {
        if (!cancelled) {
          setSheet(result);
        }
      })
      .catch((failure: unknown) => {
        if (!cancelled) {
          setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
        }
      });
    return () => {
      cancelled = true;
    };
    // Build once per modal session from the opening cart snapshot.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session?.token]);

  const printSheet = (): void => {
    const node = printRef.current;
    if (session === null || node === null || sheet === null || printing) {
      return;
    }
    setPrinting(true);
    setPrinted(null);
    const height = node.scrollHeight;
    rasterizeElement(node, PRINT_WIDTH_DOTS, height)
      .then((frame) =>
        ipcPrintRawBitmap(session.token, frame.packed, frame.widthBytes, frame.heightRows, 1),
      )
      .then((bytes) => {
        setPrinted(bytes);
      })
      .catch((failure: unknown) => {
        setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
      })
      .finally(() => {
        setPrinting(false);
      });
  };

  const checklist = (row: PrepSheetLineResult): string => {
    if (row.shares.length === 0) {
      return locale === "ar-DZ" ? "—" : "—";
    }
    return row.shares
      .map((share) => {
        const label = locale === "ar-DZ" ? share.unit_name_ar : share.unit_name_fr;
        return `${share.count} × ${label}`;
      })
      .join(" + ");
  };

  return (
    <ModalShell title={t(locale, "prep.title")} onClose={closeModal}>
      {error !== null && (
        <p className="mb-2 text-center text-sm font-semibold text-red-600">{error}</p>
      )}
      {sheet === null && error === null && (
        <p className="p-4 text-center text-sm font-semibold text-slate-500">
          {t(locale, "search.loading")}
        </p>
      )}
      {sheet !== null && (
        <div className="flex flex-col gap-2">
          {sheet.length === 0 && (
            <p className="p-4 text-center text-sm font-semibold text-slate-500">
              {t(locale, "prep.emptyCart")}
            </p>
          )}
          {sheet.map((row) => (
            <div
              key={row.product_id}
              className="rounded-lg border border-slate-200 bg-slate-50 p-2 dark:border-slate-700 dark:bg-slate-800"
            >
              <p className="text-sm font-extrabold text-slate-900 dark:text-slate-100">
                {locale === "ar-DZ" ? row.name_ar : row.name_fr}
              </p>
              <p className="font-mono text-[11px] text-slate-500">{row.sku}</p>
              <p className="mt-1 font-mono text-sm font-black text-primary-600 dark:text-sky-300">
                {checklist(row)}
              </p>
            </div>
          ))}
          <div className="flex shrink-0 gap-2">
            <button
              type="button"
              onClick={printSheet}
              disabled={printing || sheet.length === 0}
              className="flex-1 rounded-lg bg-primary-600 px-3 py-2 text-sm font-black text-white disabled:opacity-50"
            >
              {printing ? t(locale, "search.loading") : t(locale, "prep.print")}
            </button>
            {printed !== null && (
              <span className="self-center font-mono text-xs text-emerald-700">
                {printed} B
              </span>
            )}
          </div>
          {/* Off-screen 576-dot print source: laid out, never visible. */}
          <div
            aria-hidden="true"
            style={{
              position: "absolute",
              left: 0,
              top: 0,
              width: `${PRINT_WIDTH_DOTS}px`,
              visibility: "hidden",
              background: "#ffffff",
              color: "#000000",
            }}
          >
            <div ref={printRef} style={{ margin: 0, padding: "8px", width: "560px" }}>
              <p style={{ textAlign: "center", fontWeight: 900, fontSize: "20px", margin: "0 0 8px" }}>
                {t(locale, "prep.title")}
              </p>
              {sheet.map((row) => (
                <div key={row.product_id} style={{ marginBottom: "8px" }}>
                  <p style={{ fontWeight: 700, fontSize: "16px", margin: 0 }}>
                    {locale === "ar-DZ" ? row.name_ar : row.name_fr} ({row.sku})
                  </p>
                  <p style={{ fontSize: "18px", fontWeight: 900, margin: "2px 0 0" }}>
                    {checklist(row)}
                  </p>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </ModalShell>
  );
}
