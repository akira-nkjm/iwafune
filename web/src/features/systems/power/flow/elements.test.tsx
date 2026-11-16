import { ReactFlowProvider, useStoreApi } from "@xyflow/react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { smallFlow } from "@/test/flow";
import { flowNodes } from "./elements";
import { focusOf } from "./focus";
import { layoutFlow } from "./layout";

type Store = ReturnType<typeof useStoreApi>;

/** React Flow の内部の状態を取り出す（端子の位置が残るかを見る） */
function StoreProbe({ onStore }: { onStore: (s: Store) => void }) {
  onStore(useStoreApi());
  return null;
}

describe("flowNodes と React Flow", () => {
  const laid = layoutFlow(smallFlow(), new Set());
  const on = { onRowHover: () => {}, onToggleFold: () => {} };

  it("載せてノードを作り直しても、React Flow が測った端子の位置を捨てない（線が消えたままにならない）", () => {
    // `measured` の無いノードを渡し直すと React Flow は端子の位置を捨て、測り直すまで線を描けない（載せたあと線が消えた不具合）
    let store: Store | undefined;
    renderToStaticMarkup(
      <ReactFlowProvider>
        <StoreProbe onStore={(s) => (store = s)} />
      </ReactFlowProvider>,
    );
    if (!store) throw new Error("no store");
    const headings = ["部品", "条件", "解析"];
    const { setNodes, nodeLookup } = store.getState();
    setNodes(flowNodes(laid, null, null, headings, on));
    // 描いたあとに React Flow が端子を測った状態にする
    const ids = laid.cards.map((c) => c.id);
    for (const id of ids) {
      const n = nodeLookup.get(id);
      if (!n) throw new Error(id);
      nodeLookup.set(id, {
        ...n,
        internals: { ...n.internals, handleBounds: { source: [], target: [] } },
      });
    }
    for (const hover of [{ kind: "card", id: "part:SP" } as const, null]) {
      setNodes(flowNodes(laid, focusOf(laid.edges, hover), null, headings, on));
      for (const id of ids) {
        expect(store.getState().nodeLookup.get(id)?.internals.handleBounds, id).toBeDefined();
      }
    }
  });
});
