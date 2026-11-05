import adapter from "@sveltejs/adapter-static";
import { sveltekit } from "@sveltejs/kit/vite";
import { svelteTesting } from "@testing-library/svelte/vite";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [
    sveltekit({
      compilerOptions: {
        // Force runes mode, except for libraries. Can be removed in Svelte 6.
        runes: ({ filename }) => (filename.split(/[/\\]/).includes("node_modules") ? undefined : true),
      },
      // A single-page app: the files ship inside the app, and there is no server to render on.
      adapter: adapter({ fallback: "index.html" }),
    }),
  ],
  // The app's window on Windows loads this dev server from WSL, so the port is fixed.
  server: { port: 1420, strictPort: true },
  test: {
    expect: { requireAssertions: true },
    projects: [
      {
        extends: "./vite.config.ts",
        plugins: [svelteTesting()],
        test: {
          name: "components",
          environment: "happy-dom",
          include: ["src/**/*.svelte.test.ts"],
        },
      },
    ],
  },
});
