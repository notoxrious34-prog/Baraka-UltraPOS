/**
 * Contract tests for the training-sandbox environment probe + banner
 * (TASK-BRK-131).
 *
 * Pins the exact backend command name (`app_environment`), the wire
 * shape, the fail-closed browser degradation, and the banner markup in
 * FR + AR through `renderToStaticMarkup`.
 */

import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { fetchAppEnvironment } from "@/lib/tauri";
import { SandboxBannerView } from "@/components/common/SandboxBanner";
import { t } from "@/lib/i18n";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("fetchAppEnvironment", () => {
  it("maps the wire shape field-for-field over the exact command", async () => {
    const globals = globalThis as unknown as Record<string, unknown>;
    globals.window = globalThis;
    globals.__TAURI_INTERNALS__ = {};
    const { invoke } = await import("@tauri-apps/api/core");
    const invokeMock = vi.mocked(invoke);
    invokeMock.mockResolvedValue({ sandbox: true, terminal_id: "TRAIN-01" });
    const environment = await fetchAppEnvironment();
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("app_environment");
    expect(environment).toEqual({ sandbox: true, terminalId: "TRAIN-01" });
    delete globals.window;
    delete globals.__TAURI_INTERNALS__;
  });

  it("degrades fail-closed in browser mode (no Tauri marker)", async () => {
    const globals = globalThis as unknown as Record<string, unknown>;
    delete globals.window;
    delete globals.__TAURI_INTERNALS__;
    const environment = await fetchAppEnvironment();
    expect(environment.sandbox).toBe(false);
  });
});

describe("SandboxBannerView", () => {
  function esc(value: string): string {
    return value.replace(/&/g, "&amp;").replace(/'/g, "&#x27;");
  }

  it("renders the warning in French with the terminal id", () => {
    const markup = renderToStaticMarkup(
      <SandboxBannerView locale="fr-DZ" terminalId="TRAIN-01" />,
    );
    expect(markup).toContain(esc(t("fr-DZ", "app.sandboxBanner")));
    expect(markup).toContain("TRAIN-01");
    expect(markup).toContain('role="alert"');
  });

  it("renders the warning in Arabic", () => {
    const markup = renderToStaticMarkup(
      <SandboxBannerView locale="ar-DZ" terminalId="TRAIN-01" />,
    );
    expect(markup).toContain(t("ar-DZ", "app.sandboxBanner"));
  });
});
