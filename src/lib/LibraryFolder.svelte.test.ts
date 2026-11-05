import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, expect, test } from "vitest";
import LibraryFolder from "./LibraryFolder.svelte";

afterEach(() => clearMocks());

type Scan = { sources: number; unreadable: number };

function app(saved: string | null, picked: string | null, scan: () => Scan = () => ({ sources: 0, unreadable: 0 })) {
  mockIPC((cmd) => {
    switch (cmd) {
      case "library":
        return saved;
      case "choose_library":
        return picked;
      case "scan":
        return scan();
      case "games":
        return [];
    }
  });
  render(LibraryFolder);
}

test("shows the saved library folder and what its scan found", async () => {
  app("Z:\\Shadowplays", null, () => ({ sources: 612, unreadable: 3 }));
  expect(await screen.findByText("Library: Z:\\Shadowplays")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Change folder" })).toBeTruthy();
  expect(await screen.findByText("Videos: 612 · Unreadable: 3")).toBeTruthy();
});

test("asks for a folder when none is saved", async () => {
  app(null, null);
  expect(await screen.findByText("No library folder chosen yet")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Choose folder" })).toBeTruthy();
});

test("scans the folder just chosen", async () => {
  app(null, "D:\\Clips", () => ({ sources: 2, unreadable: 0 }));
  await fireEvent.click(await screen.findByRole("button", { name: "Choose folder" }));
  expect(await screen.findByText("Library: D:\\Clips")).toBeTruthy();
  expect(await screen.findByText("Videos: 2")).toBeTruthy();
});

test("keeps the folder when the dialog is cancelled", async () => {
  app("Z:\\Shadowplays", null);
  await fireEvent.click(await screen.findByRole("button", { name: "Change folder" }));
  expect(await screen.findByText("Library: Z:\\Shadowplays")).toBeTruthy();
});

test("says why a scan failed", async () => {
  app("Z:\\Shadowplays", null, () => {
    throw "FFmpeg isn't installed";
  });
  expect(await screen.findByText("Couldn't scan: FFmpeg isn't installed")).toBeTruthy();
});
