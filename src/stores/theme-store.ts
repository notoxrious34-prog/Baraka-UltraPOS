/**
 * Centralized theme store (TASK-DSN-065).
 *
 * Single owner of dark-mode state. Previously the POS Topbar kept theme in a
 * view-local `useState(false)` whose unmount/remount cycle stripped the
 * document-root `dark` class on every return to the POS view, and nothing
 * persisted across restarts. All consumers read `isDark` here; `initTheme()`
 * runs once at application startup to restore the persisted preference.
 */

import { create } from "zustand";

export type ThemeMode = "light" | "dark";

export const THEME_STORAGE_KEY = "baraka-theme";

interface ThemeState {
  theme: ThemeMode;
  isDark: boolean;
  toggleTheme: () => void;
  setTheme: (theme: ThemeMode) => void;
  initTheme: () => void;
}

function readStoredTheme(): ThemeMode {
  try {
    if (typeof localStorage === "undefined") {
      return "light";
    }
    return localStorage.getItem(THEME_STORAGE_KEY) === "dark" ? "dark" : "light";
  } catch {
    return "light";
  }
}

function persistTheme(theme: ThemeMode): void {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(THEME_STORAGE_KEY, theme);
    }
  } catch {
    // Private-mode / quota failure: state still applies for this session.
  }
}

function syncDocumentRoot(isDark: boolean): void {
  try {
    if (typeof document !== "undefined") {
      document.documentElement.classList.toggle("dark", isDark);
    }
  } catch {
    // Non-DOM test harness: state and persistence still apply.
  }
}

function applyTheme(theme: ThemeMode): Pick<ThemeState, "theme" | "isDark"> {
  persistTheme(theme);
  syncDocumentRoot(theme === "dark");
  return { theme, isDark: theme === "dark" };
}

export const useThemeStore = create<ThemeState>()((set) => ({
  theme: "light",
  isDark: false,
  toggleTheme: () =>
    set((state) => applyTheme(state.theme === "dark" ? "light" : "dark")),
  setTheme: (theme) => set(applyTheme(theme)),
  initTheme: () => set(applyTheme(readStoredTheme())),
}));
