import "@xyflow/react/dist/style.css";
import {
  Background,
  type Connection,
  ConnectionMode,
  type Edge,
  type EdgeChange,
  type Node,
  type NodeChange,
  ReactFlow,
} from "@xyflow/react";
import { useState } from "react";
import type { DiagramSelection } from "../DiagramInspector";
import { CardNode } from "./CardNode";
import { ContainerNode } from "./ContainerNode";
import { DiagramZoom, ZOOM_RANGE } from "./DiagramZoom";
import { JunctionNode } from "./JunctionNode";
import { LineStore, LineStoreContext } from "./line-store";
import { RoutedEdge } from "./RoutedEdge";

const nodeTypes = {
  card: CardNode,
  container: ContainerNode,
  junction: JunctionNode,
};
const edgeTypes = { routed: RoutedEdge };
const MINIMAP_KEY = "iwafune.diagram.minimap";

/** React Flow のキャンバス。選んだカード・辺を強調し、クリックを選択に変える */
export function DiagramCanvas({
  nodes,
  edges,
  selected,
  onSelect,
  onNodesChange,
  onEdgesChange,
  onConnect,
  onEdgesDelete,
  onNodeDragStop,
}: {
  nodes: Node[];
  edges: Edge[];
  selected: DiagramSelection;
  onSelect: (s: DiagramSelection) => void;
  onNodesChange: (c: NodeChange<Node>[]) => void;
  onEdgesChange: (c: EdgeChange<Edge>[]) => void;
  onConnect: (c: Connection) => void;
  onEdgesDelete: (e: Edge[]) => void;
  onNodeDragStop: (e: unknown, node: Node, nodes: Node[]) => void;
}) {
  const shownNodes = nodes.map((n) =>
    n.type === "card" && selected?.kind === "card"
      ? { ...n, data: { ...n.data, selected: selected.id === n.id } }
      : n,
  );
  const shownEdges = edges.map((e) => ({
    ...e,
    data: { ...e.data, selected: selected?.kind === "edge" && selected.id === e.id },
  }));
  // 線どうしで経路を分け合う場所（交差の飛び越し。line-store.ts）
  const [lines] = useState(() => new LineStore());
  // 全体図（ミニマップ）を出すか（このブラウザに覚える）
  const [minimap, setMinimap] = useState(() => {
    try {
      return localStorage.getItem(MINIMAP_KEY) === "on";
    } catch {
      return false;
    }
  });
  const toggleMinimap = (on: boolean) => {
    setMinimap(on);
    try {
      localStorage.setItem(MINIMAP_KEY, on ? "on" : "off");
    } catch {
      /* 保存できなくても切り替えはできる（覚えないだけ） */
    }
  };
  return (
    <LineStoreContext.Provider value={lines}>
      <ReactFlow
        nodes={shownNodes}
        edges={shownEdges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onEdgesDelete={onEdgesDelete}
        onNodeDragStop={onNodeDragStop}
        onNodeClick={(_, n) => {
          if (n.type === "card") onSelect({ kind: "card", id: n.id });
          else if (n.type === "junction")
            onSelect({ kind: "net", id: String((n.data as { net?: string }).net ?? "") });
        }}
        onEdgeClick={(_, e) => onSelect({ kind: "edge", id: e.id })}
        onPaneClick={() => onSelect(null)}
        connectionMode={ConnectionMode.Loose}
        deleteKeyCode={["Delete", "Backspace"]}
        snapToGrid
        snapGrid={[10, 10]}
        fitView
        // トラックパッドの 2 本指のスクロールは移動、ピンチは拡大・縮小（React Flow）。マウスのホイールの目盛りは拡大・縮小
        // （DiagramZoom の useWheelZoom。読み方は lib/wheel-input.ts）。左ドラッグ（何もない所）と中ボタンのドラッグで移動
        panOnScroll
        panOnDrag={[0, 1]}
        zoomActivationKeyCode={["Meta", "Control"]}
        minZoom={ZOOM_RANGE.min}
        maxZoom={ZOOM_RANGE.max}
        proOptions={{ hideAttribution: true }}
      >
        <Background gap={20} size={1} color="var(--color-tick)" />
        <DiagramZoom
          selected={selected}
          edges={edges}
          minimap={minimap}
          onMinimap={toggleMinimap}
        />
      </ReactFlow>
    </LineStoreContext.Provider>
  );
}
