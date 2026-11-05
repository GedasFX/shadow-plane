import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, expect, test } from "vitest";
import LibraryFolder from "./LibraryFolder.svelte";

afterEach(() => clearMocks());

function app(saved: string | null, picked: string | null) {
  mockIPC((cmd) => (cmd === "library" ? saved : cmd === "choose_library" ? picked : undefined));
  render(LibraryFolder);
}

test("shows the saved library folder", async () => {
  app("Z:\\Shadowplays", null);
  expect(await screen.findByText("Library: Z:\\Shadowplays")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Change folder" })).toBeTruthy();
});

test("asks for a folder when none is saved", async () => {
  app(null, null);
  expect(await screen.findByText("No library folder chosen yet")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Choose folder" })).toBeTruthy();
});

test("shows the folder just chosen", async () => {
  app(null, "D:\\Clips");
  await fireEvent.click(await screen.findByRole("button", { name: "Choose folder" }));
  expect(await screen.findByText("Library: D:\\Clips")).toBeTruthy();
});

test("keeps the folder when the dialog is cancelled", async () => {
  app("Z:\\Shadowplays", null);
  await fireEvent.click(await screen.findByRole("button", { name: "Change folder" }));
  expect(await screen.findByText("Library: Z:\\Shadowplays")).toBeTruthy();
});
