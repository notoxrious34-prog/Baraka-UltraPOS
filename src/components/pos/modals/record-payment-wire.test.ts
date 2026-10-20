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
  isPaymentExceedingBalance,
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
    expect(validatePaymentAmount("150", true)).toEqual({
      ok: true,
      centimes: "15000",
    });
    expect(validatePaymentAmount("150,50", true)).toEqual({
      ok: true,
      centimes: "15050",
    });
  });

  it("rejects zero, negative, and unparsable input", () => {
    expect(validatePaymentAmount("0", true)).toEqual({
      ok: false,
      key: "payment.invalidAmount",
    });
    expect(validatePaymentAmount("-5", true)).toEqual({
      ok: false,
      key: "payment.invalidAmount",
    });
    expect(validatePaymentAmount("abc", true)).toEqual({
      ok: false,
      key: "payment.invalidAmount",
    });
    expect(validatePaymentAmount("", true)).toEqual({
      ok: false,
      key: "payment.invalidAmount",
    });
  });

  it("blocks sessions lacking RECORD_CREDIT_PAYMENT (backend Unauthorized)", () => {
    expect(validatePaymentAmount("1000", false)).toEqual({
      ok: false,
      key: "payment.vendeurRestricted",
    });
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

describe("isPaymentExceedingBalance", () => {
  it("flags amounts strictly above balance when overpayment is disallowed", () => {
    expect(isPaymentExceedingBalance("12001", "12000", false)).toBe(true);
    expect(isPaymentExceedingBalance("1", "0", false)).toBe(true);
  });

  it("accepts amounts equal to or below balance", () => {
    expect(isPaymentExceedingBalance("12000", "12000", false)).toBe(false);
    expect(isPaymentExceedingBalance("8000", "12000", false)).toBe(false);
    expect(isPaymentExceedingBalance("0", "0", false)).toBe(false);
  });

  it("never flags when the role holds the overpayment grant", () => {
    expect(isPaymentExceedingBalance("99999", "12000", true)).toBe(false);
    expect(isPaymentExceedingBalance("1", "0", true)).toBe(false);
  });

  it("treats unparsable input as non-exceeding (validation owns the error)", () => {
    expect(isPaymentExceedingBalance("abc", "12000", false)).toBe(false);
    expect(isPaymentExceedingBalance("", "12000", false)).toBe(false);
    expect(isPaymentExceedingBalance("8000", "nan", false)).toBe(false);
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
