/**
 * Privacy-shield store pins (TASK-DSN-103): session-only, starts
 * unshielded, toggles symmetrically (no DOM harness in this repo —
 * store-boundary assertions only, per the pos-ui-store precedent).
 */

import { beforeEach, describe, expect, it } from "vitest";
import { usePrivacyShieldStore } from "@/stores/privacy-shield-store";

const creatorDefault = usePrivacyShieldStore.getState().shielded;

beforeEach(() => {
  usePrivacyShieldStore.setState({ shielded: false });
});

describe("privacy shield store (DSN-103)", () => {
  it("starts unshielded — figures visible by default", () => {
    expect(creatorDefault).toBe(false);
    expect(usePrivacyShieldStore.getState().shielded).toBe(false);
  });

  it("toggles symmetrically", () => {
    usePrivacyShieldStore.getState().toggleShield();
    expect(usePrivacyShieldStore.getState().shielded).toBe(true);
    usePrivacyShieldStore.getState().toggleShield();
    expect(usePrivacyShieldStore.getState().shielded).toBe(false);
  });

  it("sets explicitly", () => {
    usePrivacyShieldStore.getState().setShielded(true);
    expect(usePrivacyShieldStore.getState().shielded).toBe(true);
  });
});
