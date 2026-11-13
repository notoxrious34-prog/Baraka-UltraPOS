/**
 * Colisage display-helper pins (TASK-DSN-104): read-time breakdown only,
 * BigInt math, collapse rules mirror the domain `breakdown` degrade path.
 */

import { describe, expect, it } from "vitest";
import { colisageShares, formatColisage } from "@/lib/colisage";

const LADDER = [
  { factorMicro: "10000", nameFr: "Pièce", nameAr: "قطعة" },
  { factorMicro: "60000", nameFr: "Demi-Douzaine", nameAr: "نصف دزينة" },
  { factorMicro: "120000", nameFr: "Douzaine", nameAr: "دزينة" },
];

describe("colisageShares (DSN-104)", () => {
  it("breaks 150 pieces into 12 Douzaines + 1 Demi-Douzaine", () => {
    expect(colisageShares("1500000", "10000", LADDER)).toEqual([
      { nameFr: "Douzaine", nameAr: "دزينة", count: "12" },
      { nameFr: "Demi-Douzaine", nameAr: "نصف دزينة", count: "1" },
    ]);
  });

  it("breaks a dozen-based quantity from a dozen selling unit", () => {
    // 2.5 Douzaines = 30 pieces → 2 Douzaines + 1 Demi-Douzaine.
    expect(colisageShares("25000", "120000", LADDER)).toEqual([
      { nameFr: "Douzaine", nameAr: "دزينة", count: "2" },
      { nameFr: "Demi-Douzaine", nameAr: "نصف دزينة", count: "1" },
    ]);
  });

  it("collapses single-rung expressions the row already shows", () => {
    expect(colisageShares("50000", "10000", LADDER)).toBeNull();
  });

  it("collapses fractions under the smallest rung instead of rounding", () => {
    const cable = [
      { factorMicro: "10000", nameFr: "Mètre", nameAr: "متر" },
      { factorMicro: "100000", nameFr: "Rouleau 10m", nameAr: "لفة" },
    ];
    expect(colisageShares("125000", "10000", cable)).toBeNull();
  });

  it("refuses invalid strings and degenerate ladders", () => {
    expect(colisageShares("12.5", "10000", LADDER)).toBeNull();
    expect(colisageShares("1500000", "0", LADDER)).toBeNull();
    expect(colisageShares("0", "10000", LADDER)).toBeNull();
    expect(
      colisageShares("1500000", "10000", [
        { factorMicro: "10000", nameFr: "Pièce", nameAr: "قطعة" },
      ]),
    ).toBeNull();
  });

  it("formats locale names with × separators", () => {
    const shares = colisageShares("1500000", "10000", LADDER) ?? [];
    expect(formatColisage(shares, "fr-DZ")).toBe("12 × Douzaine + 1 × Demi-Douzaine");
    expect(formatColisage(shares, "ar-DZ")).toBe("12 × دزينة + 1 × نصف دزينة");
  });
});
