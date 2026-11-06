import { videos as listVideos, type Video } from "./api";

// The videos index, shared by the Library and the Review header; refreshed after a scan, a Save and a Discard.
export const store = $state<{ videos: Video[] }>({ videos: [] });

export async function refresh() {
  store.videos = await listVideos();
}
