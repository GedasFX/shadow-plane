import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { render, screen } from "@testing-library/svelte";
import { afterEach, expect, test } from "vitest";
import Page from "./+page.svelte";

afterEach(() => clearMocks());

test("shows the app's version", async () => {
  mockIPC((cmd) => (cmd === "plugin:app|version" ? "1.2.3" : undefined));
  render(Page);
  expect(await screen.findByText("Version 1.2.3")).toBeTruthy();
});
