/**
 * Training-sandbox banner (TASK-BRK-131).
 *
 * Self-contained: probes `app_environment` once on mount and paints an
 * unmistakable amber strip only when the backend explicitly reports the
 * `BARAKA_DATA_DIR` override. Renders nothing (not even a wrapper) in
 * live mode. The presentational `SandboxBannerView` is node-testable
 * through `renderToStaticMarkup`.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t } from "@/lib/i18n";
import { fetchAppEnvironment } from "@/lib/tauri";
import { usePosUiStore } from "@/stores/pos-ui-store";
import type { Locale } from "@/types/pos";

export function SandboxBannerView({
  locale,
  terminalId,
}: {
  locale: Locale;
  terminalId: string;
}): ReactElement {
  return (
    <div
      role="alert"
      className="flex shrink-0 items-center justify-center gap-2 bg-amber-400 px-3 py-1.5 text-center text-xs font-black text-slate-950 dark:bg-amber-500"
    >
      <span aria-hidden="true">⚠</span>
      <span>
        {t(locale, "app.sandboxBanner")} · <bdi dir="ltr">{terminalId}</bdi>
      </span>
    </div>
  );
}

export function SandboxBanner(): ReactElement | null {
  const locale = usePosUiStore((state) => state.locale);
  const [environment, setEnvironment] = useState<{ sandbox: boolean; terminalId: string } | null>(
    null,
  );

  useEffect(() => {
    let cancelled = false;
    fetchAppEnvironment()
      .then((value) => {
        if (!cancelled) {
          setEnvironment(value);
        }
      })
      .catch(() => {
        // Unreachable backend (browser preview): fail closed — no banner,
        // never a false sandbox claim.
        if (!cancelled) {
          setEnvironment({ sandbox: false, terminalId: "" });
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (environment === null || !environment.sandbox) {
    return null;
  }
  return <SandboxBannerView locale={locale} terminalId={environment.terminalId} />;
}
