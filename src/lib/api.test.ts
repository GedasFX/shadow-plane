import { clearMocks, mockConvertFileSrc } from "@tauri-apps/api/mocks";
import { afterEach, expect, test } from "vitest";
import { thumbUrl, videoUrl } from "./api";

afterEach(() => clearMocks());

test("a video's URL goes through the media protocol", () => {
  mockConvertFileSrc("windows");
  expect(videoUrl("PEAK/a b.mp4")).toBe("http://media.localhost/video%2FPEAK%2Fa%20b.mp4");
});

test("a thumbnail's URL goes through the media protocol", () => {
  mockConvertFileSrc("windows");
  expect(thumbUrl("loose.mp4")).toBe("http://media.localhost/thumb%2Floose.mp4");
});
