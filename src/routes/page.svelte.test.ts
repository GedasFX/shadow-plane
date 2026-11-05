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

test("says it is up to date when no update is found", async () => {
  mockIPC(() => undefined);
  render(Page);
  expect(await screen.findByText("Up to date")).toBeTruthy();
});

test("says why an update check failed", async () => {
  mockIPC((cmd) => {
    if (cmd === "update") throw "offline";
  });
  render(Page);
  expect(await screen.findByText("Couldn't check for updates: offline")).toBeTruthy();
});
