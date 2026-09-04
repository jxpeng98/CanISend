// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from "vitest";

type MockDragEvent = {
  payload: {
    type: "enter" | "drop";
    paths: string[];
    position: { toLogical: () => { x: number; y: number } };
  };
};

const webview = vi.hoisted(() => ({
  handler: null as ((event: MockDragEvent) => void) | null,
  unlisten: vi.fn(),
}));

vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: async (handler: (event: MockDragEvent) => void) => {
      webview.handler = handler;
      return webview.unlisten;
    },
  }),
}));

import { localFileDrop, type FileDropRejection } from "./local-file-drop";

function position(x: number, y: number) {
  return { toLogical: () => ({ x, y }) };
}

describe("localFileDrop", () => {
  beforeEach(() => {
    webview.handler = null;
    webview.unlisten.mockReset();
  });

  it("selects one supported native file dropped inside the target", async () => {
    const node = document.createElement("div");
    vi.spyOn(node, "getBoundingClientRect").mockReturnValue({
      left: 10,
      top: 10,
      right: 110,
      bottom: 60,
      width: 100,
      height: 50,
      x: 10,
      y: 10,
      toJSON: () => ({}),
    });
    const highlights: boolean[] = [];
    const dropped: string[] = [];
    const rejected: FileDropRejection[] = [];
    const action = localFileDrop(node, {
      enabled: true,
      active: () => true,
      extensions: ["typ"],
      onActiveChange: (active) => highlights.push(active),
      onDrop: (path) => dropped.push(path),
      onReject: (reason) => rejected.push(reason),
      onError: vi.fn(),
    });

    await vi.waitFor(() => expect(webview.handler).not.toBeNull());
    webview.handler!({
      payload: { type: "enter", paths: ["/tmp/profile.typ"], position: position(20, 20) },
    });
    webview.handler!({
      payload: { type: "drop", paths: ["/tmp/profile.typ"], position: position(20, 20) },
    });

    expect(highlights).toEqual([true, false]);
    expect(dropped).toEqual(["/tmp/profile.typ"]);
    expect(rejected).toEqual([]);
    action?.destroy();
    expect(webview.unlisten).toHaveBeenCalledOnce();
  });

  it("rejects ambiguous and unsupported drops without selecting a path", async () => {
    const node = document.createElement("div");
    vi.spyOn(node, "getBoundingClientRect").mockReturnValue({
      left: 0,
      top: 0,
      right: 100,
      bottom: 100,
      width: 100,
      height: 100,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    });
    const dropped: string[] = [];
    const rejected: FileDropRejection[] = [];
    const action = localFileDrop(node, {
      enabled: true,
      active: () => true,
      extensions: ["typ", "md"],
      onActiveChange: vi.fn(),
      onDrop: (path) => dropped.push(path),
      onReject: (reason) => rejected.push(reason),
      onError: vi.fn(),
    });

    await vi.waitFor(() => expect(webview.handler).not.toBeNull());
    webview.handler!({
      payload: {
        type: "drop",
        paths: ["/tmp/one.typ", "/tmp/two.md"],
        position: position(20, 20),
      },
    });
    webview.handler!({
      payload: { type: "drop", paths: ["/tmp/source.html"], position: position(20, 20) },
    });
    webview.handler!({
      payload: { type: "drop", paths: ["/tmp/outside.typ"], position: position(120, 20) },
    });

    expect(rejected).toEqual(["multiple", "unsupported"]);
    expect(dropped).toEqual([]);
    action?.destroy();
  });
});
