/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        baraka: {
          primary: "#0F3D5C",
          primaryDark: "#0A2C44",
          primaryLight: "#1A5A85",
          accent: "#F59E0B",
          accentDark: "#D97706",
          accentLight: "#FBBF24",
          steel: "#64748B",
          paper: "#F8FAFC",
          ink: "#0F172A",
        },
        primary: {
          900: "#0F3D5C",
          600: "#1E6091",
        },
        accent: {
          500: "#F59E0B",
        },
        /* Semantic design tokens (ADR-094, TASK-DSN-055). No collision:
           no top-level surface/ink/line/success/warning/danger/info key
           existed (baraka.ink is namespaced and untouched). */
        surface: {
          canvas: "var(--color-surface-canvas)",
          raised: "var(--color-surface-raised)",
          sunken: "var(--color-surface-sunken)",
        },
        ink: {
          primary: "var(--color-ink-primary)",
          secondary: "var(--color-ink-secondary)",
          inverse: "var(--color-ink-inverse)",
          muted: "var(--color-ink-muted)",
        },
        line: {
          DEFAULT: "var(--color-line-default)",
          strong: "var(--color-line-strong)",
        },
        success: { bg: "var(--color-success-bg)", fg: "var(--color-success-fg)" },
        warning: { bg: "var(--color-warning-bg)", fg: "var(--color-warning-fg)" },
        danger: { bg: "var(--color-danger-bg)", fg: "var(--color-danger-fg)" },
        info: { bg: "var(--color-info-bg)", fg: "var(--color-info-fg)" },
      },
      fontFamily: {
        sans: ["Inter", "Cairo", "system-ui", "Segoe UI", "Tahoma", "sans-serif"],
        arabic: ["Cairo", "Inter", "system-ui", "Tahoma", "sans-serif"],
        mono: ["JetBrains Mono", "ui-monospace", "SFMono-Regular", "Menlo", "monospace"],
      },
    },
  },
  plugins: [],
};
