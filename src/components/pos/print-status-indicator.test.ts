/**
 * Failure-surface pins for the post-checkout print chip (TASK-BRK-079).
 * Repo convention: source-text assertions (no DOM harness in this tree).
 * A FAILED job must render guidance (FR/AR), an aria-live region, and keep
 * the one-click reprint path wired to `documents_reprint`.
 */

// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { describe, expect, it } from "vitest";

declare const process: { cwd(): string };

const ROOT = process.cwd();
const component = readFileSync(
  join(ROOT, "src/components/pos/PrintStatusIndicator.tsx"),
  "utf8",
);
const i18n = readFileSync(join(ROOT, "src/lib/i18n.ts"), "utf8");

describe("PrintStatusIndicator failure surface", () => {
  it("renders a FAILED-only guidance banner with role=alert", () => {
    expect(component).toContain('status.status === "FAILED"');
    expect(component).toContain('role="alert"');
    expect(component).toContain('t(locale, "print.failedGuidance")');
  });
  it("announces status changes via an aria-live region", () => {
    expect(component).toContain('aria-live="polite"');
  });
  it("keeps one-click reprint wired to the outbox (documents_reprint)", () => {
    expect(component).toContain("ipcReprint(session.token, documentId)");
  });
  it("carries the FR guidance string", () => {
    expect(i18n).toContain(
      '"print.failedGuidance": "Vérifiez l\'imprimante et les paramètres, puis réimprimez."',
    );
  });
  it("carries the AR guidance string", () => {
    expect(i18n).toContain(
      '"print.failedGuidance": "تحقق من الطابعة والإعدادات، ثم أعد الطباعة."',
    );
  });
  it("stays mounted on the post-checkout done screen", () => {
    const checkout = readFileSync(
      join(ROOT, "src/components/pos/modals/CheckoutModal.tsx"),
      "utf8",
    );
    expect(checkout).toContain("<PrintStatusIndicator documentId={done.document_id} />");
  });
});
