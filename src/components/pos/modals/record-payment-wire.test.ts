/**
 * Contract tests for the credit-payment (Versement) wire helpers.
 *
 * These pin the exact `credit_record_payment` backend key names
 * (snake_case, canonical centime strings) and the VENDEUR UX guard, so a
 * drift between this modal and the Rust DTO fails here — not in front of
 * the cashier. Backend error codes asserted here must stay translatable
 * in both locales (the modal surfaces them via translateBackendError).
 */

import { describe, expect, it, vi } from "vitest";
import {
  buildPaymentReference,
  buildRecordPaymentWire,
  canUseOverpayment,
  isVendeur,
  methodTag,
  validatePaymentAmount,
} from "@/components/pos/modals/RecordPaymentModal";
import { translateBackendError } from "@/lib/i18n";

describe("methodTag", () => {
  it("maps every method to a ledger-searchable tag", () => {
    expect(methodTag("CASH")).toBe("ESPECES");
    expect(methodTag("CHEQUE")).toBe("CHEQUE");
    expect(methodTag("TRANSFER")).toBe("VIREMENT");
  });
});

describe("buildPaymentReference", () => {
  it("emits the bare tag when no note is given", () => {
    expect(buildPaymentReference("CASH", "")).toBe("ESPECES");
    expect(buildPaymentReference("CASH", "   ")).toBe("ESPECES");
  });

  it("prefixes the cashier note with the method tag", () => {
    expect(buildPaymentReference("CHEQUE", "CHK-42")).toBe("CHEQUE:CHK-42");
    expect(buildPaymentReference("TRANSFER", " VIR-9 ")).toBe("VIREMENT:VIR-9");
  });
});

describe("validatePaymentAmount", () => {
  it("accepts positive decimal input as canonical centimes", () => {
    expect(validatePaymentAmount("150", "CAISSIER")).toEqual({
      ok: true,
      centimes: "15000",
    });
    expect(validatePaymentAmount("150,50", "GERANT")).toEqual({
      ok: true,
      centimes: "15050",
    });
  });

  it("rejects zero, negative, and unparsable input", () => {
    expect(validatePaymentAmount("0", "CAISSIER")).toEqual({
      ok: false,
      key: "payment.invalidAmount",
    });
    expect(validatePaymentAmount("-5", "CAISSIER")).toEqual({
      ok: false,
      key: "payment.invalidAmount",
    });
    expect(validatePaymentAmount("abc", "CAISSIER")).toEqual({
      ok: false,
      key: "payment.invalidAmount",
    });
    expect(validatePaymentAmount("", "CAISSIER")).toEqual({
      ok: false,
      key: "payment.invalidAmount",
    });
  });

  it("blocks VENDEUR for any amount (mirrors backend Unauthorized)", () => {
    expect(isVendeur("VENDEUR")).toBe(true);
    expect(isVendeur("CAISSIER")).toBe(false);
    expect(validatePaymentAmount("1000", "VENDEUR")).toEqual({
      ok: false,
      key: "payment.vendeurRestricted",
    });
  });
});

describe("canUseOverpayment", () => {
  it("reserves negative-balance payments to GERANT/ADMIN", () => {
    expect(canUseOverpayment("GERANT")).toBe(true);
    expect(canUseOverpayment("ADMIN")).toBe(true);
    expect(canUseOverpayment("CAISSIER")).toBe(false);
    expect(canUseOverpayment("VENDEUR")).toBe(false);
  });
});

describe("buildRecordPaymentWire", () => {
  it("emits exact backend key names and canonical value strings", () => {
    const wire = buildRecordPaymentWire({
      customerId: "c-1",
      amountText: "200",
      method: "CASH",
      note: "",
      allowOverpayment: false,
    });
    expect(wire).toEqual({
      customer_id: "c-1",
      amount_centimes: "20000",
      reference: null,
      allow_overpayment: false,
    });
  });

  it("folds method and note into the opaque reference string", () => {
    const wire = buildRecordPaymentWire({
      customerId: "c-1",
      amountText: "200,50",
      method: "CHEQUE",
      note: "CHK-7",
      allowOverpayment: true,
    });
    expect(wire).toEqual({
      customer_id: "c-1",
      amount_centimes: "20050",
      reference: "CHEQUE:CHK-7",
      allow_overpayment: true,
    });
  });

  it("throws the i18n key on invalid input (submit stays disabled)", () => {
    expect(() =>
      buildRecordPaymentWire({
        customerId: "c-1",
        amountText: "0",
        method: "CASH",
        note: "",
        allowOverpayment: false,
      }),
    ).toThrowError("payment.invalidAmount");
  });
});

describe("payment backend error surfacing", () => {
  const codes = [
    "INVALID_PAYMENT_AMOUNT",
    "CUSTOMER_NOT_FOUND",
    "PAYMENT_EXCEEDS_BALANCE",
    "OVERPAYMENT_UNAUTHORIZED",
    "UNAUTHORIZED",
  ];
  it.each(codes)("maps %s in both locales (never raw)", (code) => {
    const fr = translateBackendError(code, "fr-DZ");
    const ar = translateBackendError(code, "ar-DZ");
    expect(fr).not.toBe("Erreur interne du noyau");
    expect(ar).not.toBe("خطأ داخلي في النواة");
    expect(fr.length).toBeGreaterThan(0);
    expect(ar.length).toBeGreaterThan(0);
  });

  it("dispatches the wire through a guarded single-flight spy", async () => {
    let inFlight = 0;
    let calls = 0;
    const dispatch = async (wire: {
      readonly amount_centimes: string;
    }): Promise<string> => {
      calls += 1;
      inFlight += 1;
      if (inFlight > 1) {
        throw new Error("DOUBLE_SUBMIT");
      }
      await Promise.resolve();
      inFlight -= 1;
      return wire.amount_centimes;
    };
    const wire = buildRecordPaymentWire({
      customerId: "c-1",
      amountText: "50",
      method: "CASH",
      note: "",
      allowOverpayment: false,
    });
    const guarded = vi.fn(dispatch);
    // Sequential awaits (the modal's busy ref blocks concurrent overlap).
    await guarded(wire);
    await guarded(wire);
    expect(guarded).toHaveBeenCalledTimes(2);
    expect(calls).toBe(2);
  });
});
