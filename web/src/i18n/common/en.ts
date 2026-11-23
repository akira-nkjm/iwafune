import type { commonJa } from "./ja";

export const commonEn: typeof commonJa = {
  common: {
    resizeTree: "Resize tree column",
    expandAll: "Expand all",
    collapseAll: "Collapse all",
    sortColumn: "Sort ascending, descending, then original order",
    working: {
      elapsed: (seconds: number) => `${seconds} s`,
      computing: (name: string) => `Computing ${name}`,
      layingOutDiagram: "Laying out the system diagram",
      drawing3d: "Preparing the 3D view",
    },
    loading: "Loading…",
    cancel: "Cancel",
    apply: "Apply",
    applying: "Applying…",
    add: "Add",
    remove: "Delete",
    change: "Change",
    close: "Close",
    save: "Save",
    saving: "Saving…",
    discard: "Discard",
    none: "—",
    all: "All",
    retry: "Retry",
    withNote: (main: string, note: string) => `${main} (${note})`,
    ok: "OK",
    noData: "No data",
    cannotLoadDesign: (msg: string) => `Cannot load the design: ${msg}`,
    language: "Language",
    viewHelp: {
      label: "Controls help",
      title: "View controls",
      zoom: "Zoom",
      pan: "Pan",
      rotate: "Rotate",
      fit: "Fit all",
      zoomHow:
        "Trackpad: pinch. Mouse: wheel (Ctrl / ⌘ + wheel also works). Zooms about the pointer.",
      panDiagramHow:
        "Trackpad: two-finger scroll. Mouse: middle-button drag, or left-drag on empty background.",
      pan3dHow:
        "Trackpad: two-finger scroll. Mouse: middle- or right-button drag, or Shift + left drag.",
      rotateHow: "One-finger drag, or left-button drag.",
      fitDiagramHow: "The fit button at the bottom left, or the 0 key.",
      fit3dHow: "The home button at the top right (also resets the direction).",
    },
  },
};
