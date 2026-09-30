/**
 * Confirmation-gate tests for the stock-shortage decision core
 * (TASK-BRK-033).
 *
 * Drives the EXACT gate `CheckoutModal` consumes (`evaluateStockGate`):
 * non-privileged roles are blocked with or without the toggle, privileged
 * roles are blocked until they explicitly toggle, and a clean preview
 * never blocks — whatever the role.
 */

import { describe, expect, it } from "vitest";
import { evaluateStockGate, isPrivilegedStockRole } from "@/components/pos/modals/stock-gate";

describe("isPrivilegedStockRole", () => {
  it("grants ADMIN and GERANT only", () => {
    expect(isPrivilegedStockRole("ADMIN")).toBe(true);
    expect(isPrivilegedStockRole("GERANT")).toBe(true);
    expect(isPrivilegedStockRole("VENDEUR")).toBe(false);
    expect(isPrivilegedStockRole("CAISSIER")).toBe(false);
    expect(isPrivilegedStockRole("")).toBe(false);
    expect(isPrivilegedStockRole("admin")).toBe(false);
  });
});

describe("evaluateStockGate", () => {
  it("hard-blocks VENDEUR with shortages even when the toggle is on", () => {
    const gate = evaluateStockGate({ role: "VENDEUR", warningCount: 2, allowNegativeStock: true });
    expect(gate.hasShortage).toBe(true);
    expect(gate.isPrivileged).toBe(false);
    expect(gate.blocked).toBe(true);
    expect(gate.needsConfirmation).toBe(false);
  });

  it("hard-blocks CAISSIER with shortages with or without the toggle", () => {
    expect(
      evaluateStockGate({ role: "CAISSIER", warningCount: 1, allowNegativeStock: false }).blocked,
    ).toBe(true);
    expect(
      evaluateStockGate({ role: "CAISSIER", warningCount: 1, allowNegativeStock: true }).blocked,
    ).toBe(true);
  });

  it("blocks ADMIN until the explicit toggle, then releases", () => {
    const unconfirmed = evaluateStockGate({
      role: "ADMIN",
      warningCount: 1,
      allowNegativeStock: false,
    });
    expect(unconfirmed.needsConfirmation).toBe(true);
    expect(unconfirmed.blocked).toBe(true);
    const confirmed = evaluateStockGate({
      role: "ADMIN",
      warningCount: 1,
      allowNegativeStock: true,
    });
    expect(confirmed.needsConfirmation).toBe(true);
    expect(confirmed.blocked).toBe(false);
  });

  it("applies the same toggle rule to GERANT", () => {
    expect(
      evaluateStockGate({ role: "GERANT", warningCount: 3, allowNegativeStock: false }).blocked,
    ).toBe(true);
    expect(
      evaluateStockGate({ role: "GERANT", warningCount: 3, allowNegativeStock: true }).blocked,
    ).toBe(false);
  });

  it("never blocks a clean preview, whatever the role or toggle", () => {
    for (const role of ["ADMIN", "GERANT", "VENDEUR", "CAISSIER", ""]) {
      for (const allowNegativeStock of [false, true]) {
        const gate = evaluateStockGate({ role, warningCount: 0, allowNegativeStock });
        expect(gate.hasShortage).toBe(false);
        expect(gate.blocked).toBe(false);
        expect(gate.needsConfirmation).toBe(false);
      }
    }
  });

  it("treats unknown roles as non-privileged", () => {
    const gate = evaluateStockGate({ role: "SUPER", warningCount: 1, allowNegativeStock: true });
    expect(gate.isPrivileged).toBe(false);
    expect(gate.blocked).toBe(true);
  });
});
