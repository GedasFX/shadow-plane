// Prettier formats .svelte files only; Biome does the rest. Same style for both.
/** @type {import("prettier").Config} */
export default {
  useTabs: false,
  tabWidth: 2,
  printWidth: 120,
  singleQuote: false,
  trailingComma: "all",
  plugins: ["prettier-plugin-svelte"],
  overrides: [{ files: "*.svelte", options: { parser: "svelte" } }],
};
