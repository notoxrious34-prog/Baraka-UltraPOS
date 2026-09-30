/**
 * First-run setup screen (shown instead of login while the users table is
 * empty). Mirrors the LoginScreen structure, styling, and i18n pattern.
 * On success the operator enters the authenticated shell directly through
 * the standard session flow (no separate login step).
 */

import { useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { t, translateBackendError } from "@/lib/i18n";

export function FirstRunSetupScreen(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const toggleLocale = usePosUiStore((state) => state.toggleLocale);
  const bootstrap = useSessionStore((state) => state.bootstrap);
  const authBusy = useSessionStore((state) => state.authBusy);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);

  const submit = (): void => {
    if (authBusy) {
      return;
    }
    if (password !== confirmPassword) {
      setError(t(usePosUiStore.getState().locale, "setup.mismatch"));
      return;
    }
    setError(null);
    bootstrap(username, password).catch((failure: unknown) => {
      const code = typeof failure === "string" ? failure : "INTERNAL_ERROR";
      setError(translateBackendError(code, usePosUiStore.getState().locale));
    });
  };

  return (
    <div className="flex h-full flex-col items-center justify-center gap-4 bg-primary-900 p-6">
      <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-accent-500 text-3xl font-black text-slate-950">
        ب
      </div>
      <h1 className="text-2xl font-black text-white">{t(locale, "setup.title")}</h1>
      <p className="text-sm text-white/70">{t(locale, "setup.hint")}</p>
      <form
        className="flex w-full max-w-xs flex-col gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <label className="flex flex-col gap-1 text-start">
          <span className="text-xs font-bold text-white/80">{t(locale, "setup.username")}</span>
          <input
            value={username}
            onChange={(event) => {
              setUsername(event.target.value);
            }}
            autoComplete="username"
            className="h-11 rounded-lg border border-white/20 bg-white/10 px-3 text-sm font-bold text-white outline-none placeholder:text-white/40 focus:border-accent-500"
          />
        </label>
        <label className="flex flex-col gap-1 text-start">
          <span className="text-xs font-bold text-white/80">{t(locale, "setup.password")}</span>
          <input
            value={password}
            onChange={(event) => {
              setPassword(event.target.value);
            }}
            type="password"
            autoComplete="new-password"
            className="h-11 rounded-lg border border-white/20 bg-white/10 px-3 text-sm font-bold text-white outline-none placeholder:text-white/40 focus:border-accent-500"
          />
        </label>
        <label className="flex flex-col gap-1 text-start">
          <span className="text-xs font-bold text-white/80">
            {t(locale, "setup.confirmPassword")}
          </span>
          <input
            value={confirmPassword}
            onChange={(event) => {
              setConfirmPassword(event.target.value);
            }}
            type="password"
            autoComplete="new-password"
            className="h-11 rounded-lg border border-white/20 bg-white/10 px-3 text-sm font-bold text-white outline-none placeholder:text-white/40 focus:border-accent-500"
          />
        </label>
        {error !== null && (
          <p className="rounded-md bg-red-900/60 px-3 py-2 text-sm font-bold text-red-100">{error}</p>
        )}
        <button
          type="submit"
          disabled={authBusy}
          className="h-12 rounded-lg bg-accent-500 text-base font-black text-slate-950 hover:bg-amber-400 disabled:opacity-50"
        >
          {authBusy ? t(locale, "setup.loading") : t(locale, "setup.submit")}
        </button>
        <button
          type="button"
          onClick={toggleLocale}
          className="h-10 rounded-lg bg-white/10 text-sm font-bold text-white hover:bg-white/20"
        >
          {t(locale, "topbar.lang")}
        </button>
      </form>
    </div>
  );
}
