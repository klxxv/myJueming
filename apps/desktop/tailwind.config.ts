import type { Config } from "tailwindcss";

// Keep the device theme (including eye comfort) as the single source of truth.
const surfaces = ["app", "raised", "subtle", "muted", "hover", "disabled", "input", "green-soft", "green-selected", "warm-soft", "cool-soft", "code", "mark", "danger-soft", "danger-strong", "success-soft", "success-strong"];
export default {
  content: { relative: true, files: ["./index.html", "./src/**/*.{vue,ts}"] },
  darkMode: ["selector", '[data-theme="dark"]'],
  // Preserve native controls and existing macOS typography during migration.
  corePlugins: { preflight: false },
  theme: {
    extend: {
      colors: {
        ...Object.fromEntries(surfaces.map(name => [name, `var(--surface-${name})`])),
        paper: "var(--paper)",
        line: "var(--line)",
        ink: { 900: "var(--ink-900)", 700: "var(--ink-700)", 500: "var(--ink-500)" },
        accent: { DEFAULT: "var(--green-700)", strong: "var(--green-900)", solid: "var(--accent-solid)", subtle: "var(--green-050)", soft: "var(--green-100)" },
        target: "var(--target-soft)",
      },
      boxShadow: { panel: "var(--shadow-soft)" },
      fontFamily: { sans: ["var(--jm-font-ui)"], mono: ["var(--jm-font-mono)"] },
    },
  },
  plugins: [],
} satisfies Config;
