/**
 * About tab (DSN-047): official brand showcase — golden logo, version,
 * offline-first trust credentials. Read-only; no session required.
 */

import type { ReactElement } from "react";
import { t } from "@/lib/i18n";
import type { Locale } from "@/types/pos";
import barakaLogo from "@/assets/logo/baraka-logo.png";

export function AboutTab({ locale }: { locale: Locale }): ReactElement {
  return (
    <div className="flex min-h-0 flex-1 flex-col items-center justify-start gap-3 overflow-y-auto p-6 text-center">
      <img
        src={barakaLogo}
        alt="Baraka"
        className="h-32 w-32 select-none object-contain drop-shadow-md"
      />
      <h2 className="text-xl font-black">{t(locale, "app.title")}</h2>
      <p className="text-sm font-bold text-amber-700 dark:text-amber-300">
        {t(locale, "brand.tagline")}
      </p>
      <p className="rounded-full bg-slate-200 px-3 py-1 text-xs font-bold text-slate-700 dark:bg-slate-700 dark:text-slate-200">
        {t(locale, "about.version")}
      </p>
      <p className="max-w-sm text-xs text-slate-500 dark:text-slate-400">
        {t(locale, "about.offline")}
      </p>
      <p className="text-[11px] text-slate-400 dark:text-slate-500">
        {t(locale, "about.copyright")}
      </p>
    </div>
  );
}
