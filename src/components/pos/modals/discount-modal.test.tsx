/**
 * Discount modal per-unit pins (TASK-DSN-097): source-text pins (no jsdom).
 * The F5 line editor is per unit: relabeled copy FR/AR, live computed line
 * figure, and a pre-wire bound gate (engine `LineDiscountExceedsUnitPrice`
 * stays the backstop, surfaced generically as INVALID_INPUT).
 */

// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { describe, expect, it } from "vitest";

// Minimal ambient typing for the single Node global used below.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

const MODAL_SRC = readSrc("src", "components", "pos", "modals", "DiscountModal.tsx");
const STORE_SRC = readSrc("src", "stores", "cart-store.ts");
const I18N_SRC = readSrc("src", "lib", "i18n.ts");

describe("discount modal per-unit surface", () => {
  it("labels the line editor per unit (not off-total)", () => {
    expect(MODAL_SRC).toContain('t(locale, "discount.linePerUnit")');
  });

  it("states the amount hits EACH unit (hint copy)", () => {
    expect(MODAL_SRC).toContain('t(locale, "discount.linePerUnitHint")');
  });

  it("shows a live computed line figure before confirm", () => {
    expect(MODAL_SRC).toContain('t(locale, "discount.lineTotal")');
    expect(MODAL_SRC).toContain("liveLine");
  });

  it("gates discount > unit price before any IPC (engine backstop untouched)", () => {
    expect(MODAL_SRC).toContain('t(locale, "discount.exceedsUnitPrice")');
    expect(MODAL_SRC).toContain("BigInt(line) > BigInt(selectedUnitPrice)");
    expect(MODAL_SRC).toContain("LineDiscountExceedsUnitPrice");
  });

  it("keeps the global editor untouched in meaning", () => {
    expect(MODAL_SRC).toContain('t(locale, "discount.global")');
    expect(I18N_SRC).toContain('"discount.global": "Remise globale (DZD)"');
    expect(I18N_SRC).toContain('"discount.global": "التخفيض العام (د.ج)"');
  });
});

describe("cart preview per-unit math pins", () => {
  it("derives the net PU and the line total from the typed per-unit amount", () => {
    expect(STORE_SRC).toContain("export function previewDiscountedUnitPrice");
    expect(STORE_SRC).toContain("qty * discounted unit price");
  });
});

describe("discount i18n copy (FR/AR)", () => {
  it("labels the line editor per unit in both locales", () => {
    expect(I18N_SRC).toContain('"discount.linePerUnit": "Remise ligne / unité (DZD)"');
    expect(I18N_SRC).toContain('"discount.linePerUnit": "تخفيض السطر / للوحدة (د.ج)"');
  });

  it("hints EACH-unit application in both locales", () => {
    expect(I18N_SRC).toContain("CHAQUE unité");
    expect(I18N_SRC).toContain("ينطبق المبلغ على كل وحدة");
  });

  it("refuses over-price discounts with a bound message in both locales", () => {
    expect(I18N_SRC).toContain('"discount.exceedsUnitPrice": "La remise ne peut pas dépasser le prix unitaire"');
    expect(I18N_SRC).toContain('"discount.exceedsUnitPrice": "لا يمكن أن يتجاوز التخفيض سعر الوحدة"');
  });

  it("names the returns per-unit badge in both locales", () => {
    expect(I18N_SRC).toContain('"returns.perUnitDiscount": "Remise /u"');
    expect(I18N_SRC).toContain('"returns.perUnitDiscount": "تخفيض / للوحدة"');
  });

  it("names the document per-unit caption in both locales", () => {
    expect(I18N_SRC).toContain('"documents.perUnitDiscount": "dont remise /u"');
    expect(I18N_SRC).toContain('"documents.perUnitDiscount": "منها تخفيض / للوحدة"');
  });
});
