import "@xyflow/react/dist/style.css";
import { Background, ReactFlow } from "@xyflow/react";
import { useMemo, useState } from "react";
import type { DesignView, PowerPageView } from "@/api/types";
import { DiagramZoom, ZOOM_RANGE } from "@/features/diagram/canvas/DiagramZoom";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { flowEdges, flowNodes } from "./elements";
import { FlowCard, FlowFoldCard, FlowHeading } from "./FlowCard";
import { FlowDetail } from "./FlowDetail";
import { type FlowHover, focusOf, neighbours } from "./focus";
import { foldEdges, layoutFlow } from "./layout";

const nodeTypes = { flowCard: FlowCard, flowFold: FlowFoldCard, flowHeading: FlowHeading };
const NO_FOLDS: ReadonlyMap<string, string> = new Map();

/**
 * 電源のページの「フロー」タブ: 部品・条件と設定・解析のカードを、値の行 → 読む解析、解析の結果 → 使う解析の線で
 * つなぐ（カード・行・線は API の `PowerPageView.flow`。置き方は `layout.ts`）。載せると関係するものを強調し、押すと右に
 * 詳細。ズーム・移動は系統図と同じ操作（`DiagramZoom`）。この版は読むだけ。
 */
export function PowerFlow({ page, design }: { page: PowerPageView; design?: DesignView }) {
  const t = useT().systems.flow;
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(() => new Set());
  const [hover, setHover] = useState<FlowHover>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [minimap, setMinimap] = useState(false);
  const laid = useMemo(() => layoutFlow(page.flow, expanded), [page.flow, expanded]);
  const focus = useMemo(() => focusOf(laid.edges, hover), [laid.edges, hover]);
  const nodes = flowNodes(laid, focus, selected, [t.parts, t.conditions, t.analyses], {
    onRowHover: (card, row) =>
      setHover(row ? { kind: "row", card, row } : { kind: "card", id: card }),
    onToggleFold: (fold) =>
      setExpanded((was) => {
        const next = new Set(was);
        if (next.has(fold)) next.delete(fold);
        else next.add(fold);
        return next;
      }),
  });
  const edges = flowEdges(laid.edges, focus);
  if (page.flow.nodes.length === 0) return <Empty>{t.empty}</Empty>;
  const chosen = page.flow.nodes.find((n) => n.id === selected);
  const all = foldEdges(page.flow.edges, NO_FOLDS);
  const near = chosen ? neighbours(all, chosen.id) : { from: [], to: [] };
  const byId = (ids: string[]) => page.flow.nodes.filter((n) => ids.includes(n.id));
  return (
    <div className="flex h-[calc(100vh-232px)] min-h-[520px] gap-3">
      <Panel pad={false} className="flex min-w-0 flex-1 flex-col overflow-hidden">
        <div className="px-4 pt-3">
          <PanelHeader title={t.title} caption={t.caption} right={<Legend />} />
        </div>
        <div className="min-h-0 flex-1">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={nodeTypes}
            nodesDraggable={false}
            nodesConnectable={false}
            elementsSelectable={false}
            onNodeMouseEnter={(_, n) => {
              if (n.type !== "flowHeading") setHover({ kind: "card", id: n.id });
            }}
            onNodeMouseLeave={() => setHover(null)}
            onEdgeMouseEnter={(_, e) => setHover({ kind: "edge", id: e.id })}
            onEdgeMouseLeave={() => setHover(null)}
            onNodeClick={(_, n) => {
              if (n.type === "flowCard") setSelected(n.id);
            }}
            onPaneClick={() => setSelected(null)}
            fitView
            fitViewOptions={{ padding: 0.08 }}
            panOnScroll
            zoomActivationKeyCode={["Meta", "Control"]}
            minZoom={ZOOM_RANGE.min}
            maxZoom={ZOOM_RANGE.max}
            proOptions={{ hideAttribution: true }}
          >
            <Background gap={20} size={1} color="var(--color-tick)" />
            <DiagramZoom
              selected={selected ? { kind: "card", id: selected } : null}
              edges={edges}
              minimap={minimap}
              onMinimap={setMinimap}
            />
          </ReactFlow>
        </div>
      </Panel>
      {chosen && (
        <Panel pad={false} className="w-[360px] shrink-0 overflow-hidden">
          <FlowDetail
            node={chosen}
            result={page.analyses.find((a) => a.result.id === chosen.analysis)?.result}
            from={byId(near.from)}
            to={byId(near.to)}
            design={design}
            onSelect={setSelected}
            onClose={() => setSelected(null)}
          />
        </Panel>
      )}
    </div>
  );
}

/** 線の凡例（値の線は実線、解析の結果の線は破線） */
function Legend() {
  const t = useT().systems.flow;
  return (
    <div className="flex items-center gap-3 text-[11px] text-ink-3">
      <span className="flex items-center gap-1.5">
        <span className="inline-block h-px w-5 bg-ink-4" />
        {t.legendInput}
      </span>
      <span className="flex items-center gap-1.5">
        <span className="inline-block w-5 border-ink-3 border-t-[1.5px] border-dashed" />
        {t.legendResult}
      </span>
    </div>
  );
}
