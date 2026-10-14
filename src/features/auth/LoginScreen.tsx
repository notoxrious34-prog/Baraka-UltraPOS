/**
 * Industrial login screen (shown when no session is active).
 */

import { useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { t, translateBackendError } from "@/lib/i18n";
import barakaLogo from "@/assets/logo/baraka-logo.png";

export function LoginScreen(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const toggleLocale = usePosUiStore((state) => state.toggleLocale);
  const login = useSessionStore((state) => state.login);
  const authBusy = useSessionStore((state) => state.authBusy);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);

  const submit = (): void => {
    if (authBusy) {
      return;
    }
    setError(null);
    login(username, password).catch((failure: unknown) => {
      const code = typeof failure === "string" ? failure : "INTERNAL_ERROR";
      setError(translateBackendError(code, usePosUiStore.getState().locale));
    });
  };

  return (
    <div className="flex h-full flex-col items-center justify-center gap-4 bg-primary-900 p-6">
      <img
        src={barakaLogo}
        alt="Baraka"
        className="h-28 w-28 select-none object-contain drop-shadow-lg"
      />
      <h1 className="text-2xl font-black text-white">{t(locale, "app.title")}</h1>
      <p className="text-sm text-amber-200/90">{t(locale, "brand.tagline")}</p>
      <p className="text-sm text-white/70">{t(locale, "login.hint")}</p>
      <form
        className="flex w-full max-w-xs flex-col gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <label className="flex flex-col gap-1 text-start">
          <span className="text-xs font-bold text-white/80">{t(locale, "login.username")}</span>
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
          <span className="text-xs font-bold text-white/80">{t(locale, "login.password")}</span>
          <input
            value={password}
            onChange={(event) => {
              setPassword(event.target.value);
            }}
            type="password"
            autoComplete="current-password"
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
          {authBusy ? t(locale, "login.loading") : t(locale, "login.submit")}
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
