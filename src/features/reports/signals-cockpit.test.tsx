/**
 * Unit tests for the signal-cockpit view (TASK-DSN-110).
 *
 * The pure `SignalsCockpitView` renders through `renderToStaticMarkup`
 * with explicit props (node-only harness, no DOM). Pins: all five
 * sections in FR + AR, window switcher labels, BigInt margin
 * formatting, money/qty helpers, loading and retry states.
 */

import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { SignalsCockpitView, formatMarginBp } from "@/features/reports/SignalsCockpitReport";
import { t, translateBackendError } from "@/lib/i18n";
import type { SignalsFeed } from "@/types/reports";

function feed(): SignalsFeed {
  return {
    profile: "apparel_wholesale_retail",
    brokenRuns: [
      {
        modelId: "M-NOIR",
        cells: [{ axis1Value: "M", axis2Value: "Noir", siblingUnitsSold: "20" }],
      },
    ],
    deadStock: [
      { productId: "P-OLD", qtyOnHandMicro: "25000", daysSinceLastMovement: "90" },
    ],
    velocity: { unitsPerDay: "0", isSlow: true, trendBps: "-500" },
    modelMargins: [
      {
        modelId: "M-NOIR",
        revenueCentimes: "375000",
        cogsCentimes: "225000",
        marginBps: "4000",
      },
    ],
    aging: {
      bucket030Centimes: "100000",
      bucket3160Centimes: "0",
      bucket6190Centimes: "0",
      bucket90PlusCentimes: "0",
    },
  };
}

describe("SignalsCockpitView", () => {
  it("formats basis points with BigInt only", () => {
    expect(formatMarginBp("4000")).toBe("40.00 %");
    expect(formatMarginBp("-500")).toBe("-5.00 %");
    expect(formatMarginBp("0")).toBe("0.00 %");
  });

  it("renders all five sections in French", () => {
    const markup = renderToStaticMarkup(
      <SignalsCockpitView
        locale="fr-DZ"
        feed={feed()}
        loading={false}
        error={null}
        windowDays={30}
        onWindowChange={() => {}}
        onRetry={() => {}}
      />,
    );
    expect(markup).toContain(t("fr-DZ", "rep.signalsBrokenRuns"));
    expect(markup).toContain(t("fr-DZ", "rep.signalsDeadStock"));
    expect(markup).toContain(t("fr-DZ", "rep.signalsVelocity"));
    expect(markup).toContain(t("fr-DZ", "rep.signalsMargins"));
    expect(markup).toContain(t("fr-DZ", "rep.signalsAging"));
    expect(markup).toContain("M-NOIR");
    expect(markup).toContain("40.00 %");
    expect(markup).toContain("2.5");
    expect(markup).toContain(t("fr-DZ", "rep.signalsSlow"));
    expect(markup).toContain("30");
  });

  it("renders all five sections in Arabic with DZD suffix", () => {
    const markup = renderToStaticMarkup(
      <SignalsCockpitView
        locale="ar-DZ"
        feed={feed()}
        loading={false}
        error={null}
        windowDays={7}
        onWindowChange={() => {}}
        onRetry={() => {}}
      />,
    );
    expect(markup).toContain(t("ar-DZ", "rep.signalsBrokenRuns"));
    expect(markup).toContain(t("ar-DZ", "rep.signalsDeadStock"));
    expect(markup).toContain(t("ar-DZ", "rep.signalsVelocity"));
    expect(markup).toContain(t("ar-DZ", "rep.signalsMargins"));
    expect(markup).toContain(t("ar-DZ", "rep.signalsAging"));
    expect(markup).toContain("د.ج");
  });

  it("shows loading and retry states", () => {
    const loading = renderToStaticMarkup(
      <SignalsCockpitView
        locale="fr-DZ"
        feed={null}
        loading={true}
        error={null}
        windowDays={30}
        onWindowChange={() => {}}
        onRetry={() => {}}
      />,
    );
    expect(loading).toContain(t("fr-DZ", "rep.loading"));
    const failed = renderToStaticMarkup(
      <SignalsCockpitView
        locale="fr-DZ"
        feed={null}
        loading={false}
        error="SIGNAL_STORE_UNAVAILABLE"
        windowDays={30}
        onWindowChange={() => {}}
        onRetry={() => {}}
      />,
    );
    expect(failed).toContain(translateBackendError("SIGNAL_STORE_UNAVAILABLE", "fr-DZ"));
    expect(failed).toContain(t("fr-DZ", "sup.retry"));
  });
});
