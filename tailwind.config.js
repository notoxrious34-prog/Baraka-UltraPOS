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
