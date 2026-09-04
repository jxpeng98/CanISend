import { getCurrentWebview } from "@tauri-apps/api/webview";

export type FileDropRejection = "multiple" | "unsupported";

type LocalFileDropOptions = {
  enabled: boolean;
  active: () => boolean;
  extensions: readonly string[];
  onActiveChange: (active: boolean) => void;
  onDrop: (path: string) => void;
  onReject: (reason: FileDropRejection) => void;
  onError: (error: unknown) => void;
};

export function localFileDrop(node: HTMLElement, options: LocalFileDropOptions) {
  if (!options.enabled) return;

  const extensions = new Set(options.extensions.map((extension) => extension.toLowerCase()));
  let destroyed = false;
  let highlighted = false;
  let unlisten: (() => void) | undefined;

  function setHighlighted(next: boolean): void {
    if (highlighted === next) return;
    highlighted = next;
    options.onActiveChange(next);
  }

  void getCurrentWebview()
    .onDragDropEvent((event) => {
      if (!options.active()) {
        setHighlighted(false);
        return;
      }
      if (event.payload.type === "leave") {
        setHighlighted(false);
        return;
      }

      const point = event.payload.position.toLogical(window.devicePixelRatio || 1);
      const bounds = node.getBoundingClientRect();
      const inside =
        point.x >= bounds.left &&
        point.x <= bounds.right &&
        point.y >= bounds.top &&
        point.y <= bounds.bottom;

      if (event.payload.type !== "drop") {
        setHighlighted(inside);
        return;
      }

      setHighlighted(false);
      if (!inside) return;
      if (event.payload.paths.length !== 1) {
        options.onReject("multiple");
        return;
      }

      const path = event.payload.paths[0];
      const filename = path.split(/[\\/]/u).at(-1) ?? "";
      const extension = filename.includes(".") ? (filename.split(".").at(-1) ?? "") : "";
      if (!extensions.has(extension.toLowerCase())) {
        options.onReject("unsupported");
        return;
      }
      options.onDrop(path);
    })
    .then((stop) => {
      if (destroyed) stop();
      else unlisten = stop;
    })
    .catch((error: unknown) => {
      if (!destroyed) options.onError(error);
    });

  return {
    destroy() {
      destroyed = true;
      setHighlighted(false);
      unlisten?.();
    },
  };
}
