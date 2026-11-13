/**
 * Credit-equation helper pins (TASK-DSN-103, ADR-111/112 print rule).
 *
 * The printed `Versement` is the non-credit tender (`Montant − |delta|`),
 * never the raw paid column; absent snapshots and tampered figures yield
 * `null` so surfaces collapse the block instead of printing a lie.
 * BigInt only — no `Number`, no floats.
 */

import { describe, expect, it } from "vitest";
import { creditEquationFromDocument } from "@/lib/credit-equation";

describe("creditEquationFromDocument (DSN-103)", () => {
  it("builds the printed equation for a credit sale", () => {
    // 200.00 sale, 50.00 cash + 150.00 credit leg.
    const equation = creditEquationFromDocument({
      total_ttc_centimes: "20000",
      credit_solde_before_centimes: "0",
      credit_solde_after_centimes: "15000",
      credit_delta_centimes: "15000",
    });
    expect(equation).toEqual({
      ancien: "0",
      montant: "20000",
      versement: "5000",
      nouveau: "15000",
    });
  });

  it("builds the mirrored equation for an AVOIR refund", () => {
    // Ancien 0, AVOIR of 200.00 fully refunded on credit: delta −200.00.
    const equation = creditEquationFromDocument({
      total_ttc_centimes: "20000",
      credit_solde_before_centimes: "0",
      credit_solde_after_centimes: "-20000",
      credit_delta_centimes: "-20000",
    });
    expect(equation).toEqual({
      ancien: "0",
      montant: "20000",
      versement: "0",
      nouveau: "-20000",
    });
  });

  it("collapses cash-only and legacy rows to null, never zero", () => {
    expect(
      creditEquationFromDocument({
        total_ttc_centimes: "20000",
        credit_solde_before_centimes: null,
        credit_solde_after_centimes: null,
        credit_delta_centimes: null,
      }),
    ).toBeNull();
    expect(
      creditEquationFromDocument({
        total_ttc_centimes: "20000",
        credit_solde_before_centimes: "0",
        credit_solde_after_centimes: null,
        credit_delta_centimes: "15000",
      }),
    ).toBeNull();
  });

  it("refuses tampered figures instead of printing a lie", () => {
    // Nouveau does not close: 0 + 20000 − 5000 = 15000 ≠ 99999.
    expect(
      creditEquationFromDocument({
        total_ttc_centimes: "20000",
        credit_solde_before_centimes: "0",
        credit_solde_after_centimes: "99999",
        credit_delta_centimes: "15000",
      }),
    ).toBeNull();
  });

  it("refuses malformed wire strings", () => {
    expect(
      creditEquationFromDocument({
        total_ttc_centimes: "20.000",
        credit_solde_before_centimes: "0",
        credit_solde_after_centimes: "15000",
        credit_delta_centimes: "15000",
      }),
    ).toBeNull();
  });
});
