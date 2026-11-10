import { LayoutGrid, Redo2, RotateCcw, Undo2 } from "lucide-react";
import { useState } from "react";
import { useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { SkeletonCanvas } from "@/kit/Loading";
import { QueryState } from "@/kit/QueryState";
import { CanvasFrame } from "@/shell/CanvasFrame";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import { DiagramCanvas } from "./DiagramCanvas";
import { DiagramInspector, type DiagramSelection } from "./DiagramInspector";
import { DiagramLegend } from "./DiagramLegend";
import { DiagramToolbar } from "./DiagramToolbar";
import { NewNetDialog } from "./NewNetDialog";
import type { Layer } from "./power";
import { useDiagramConnect } from "./use-diagram-connect";
import { useDiagramFlow, useDiagramModel } from "./use-diagram-flow";
import { useUndoKeys } from "./use-undo-keys";
import { useViewEdits, useViewUndo } from "./view-history";

/**
 * 設計 / システム図: サブシステムの枠の中に部品のカード、組立品は子のカードを中に持つ。給電・信号・GND の層を重ねて
 * （どの組み合わせでも）見られる。配置は layout.ts（枠を縦横に置く探索・格子の上の線の経路探索・線の飛び越し）、手で動かした位置と
 * 枠の大きさは set_view_layout、線を選んでドラッグした経路は set_view_edge で view/electrical.toml に保存する。
 * ポートどうしをドラッグで繋ぐと SetPort。右の詳細欄は欄の見出しのボタンで畳め、畳むと右端の縦のつまみから開く
 * （開閉はこのブラウザに覚える）。
 */
export function DiagramPage() {
  const design = useDesign();
  const t = useT();
  const td = t.diagram;
  const [layers, setLayers] = useState<ReadonlySet<Layer>>(new Set<Layer>(["power", "signal"]));
  const [merge, setMerge] = useState(true);
  const { editRoute } = useViewEdits();
  const history = useViewUndo();
  const [inspectorOpen, setInspectorOpen] = useState(() => {
    try {
      return localStorage.getItem("iwafune.diagram.inspector") !== "hidden";
    } catch {
      return true;
    }
  });
  const toggleInspector = () =>
    setInspectorOpen((open) => {
      try {
        localStorage.setItem("iwafune.diagram.inspector", open ? "hidden" : "visible");
      } catch {
        /* ブラウザの保存領域が使えなくても開閉はできる（覚えないだけ） */
      }
      return !open;
    });
  const [selected, setSelected] = useState<DiagramSelection>(null);
  const d = design.data;
  const { model, layout } = useDiagramModel(d, layers, merge);
  const flow = useDiagramFlow(d, model, layout);
  // ⌘Z・⇧⌘Z（Ctrl+Z・Ctrl+Y）で配置の編集を元に戻す・やり直す。戻す前に、手で動かした印と保存待ちの位置を捨てる
  const undo = () => {
    flow.resetMoved();
    history.undo();
  };
  const redo = () => {
    flow.resetMoved();
    history.redo();
  };
  useUndoKeys(undo, redo);
  const connect = useDiagramConnect(model);
  return (
    <>
      <PageHeader
        eyebrow={td.eyebrow}
        title={td.title}
        subtitle={
          model
            ? td.subtitle(model.cards.length, model.containers.length, model.edges.length)
            : undefined
        }
      />
      {/* 設計から作った図（配置は空き時間に計算する）ができるまでは、図のスケルトン */}
      <QueryState
        query={{
          data: d && model && layout ? { design: d, model } : undefined,
          error: design.error,
          refetch: design.refetch,
        }}
        className="flex min-h-0 flex-1 flex-col"
        skeleton={<SkeletonCanvas />}
      >
        {(shown) => (
          <CanvasFrame
            toolbar={
              <>
                <DiagramToolbar
                  layers={layers}
                  onLayers={setLayers}
                  merge={merge}
                  onMerge={setMerge}
                />
                <Button
                  size="icon-sm"
                  variant="outline"
                  title={td.undoTitle}
                  aria-label={td.undo}
                  disabled={!history.canUndo}
                  onClick={undo}
                >
                  <Undo2 size={13} />
                </Button>
                <Button
                  size="icon-sm"
                  variant="outline"
                  title={td.redoTitle}
                  aria-label={td.redo}
                  disabled={!history.canRedo}
                  onClick={redo}
                >
                  <Redo2 size={13} />
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  title={td.autoLayoutTitle}
                  onClick={flow.autoLayout}
                >
                  <LayoutGrid size={13} />
                  {td.autoLayout}
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  title={td.resetLayoutTitle}
                  onClick={() => {
                    flow.resetMoved();
                    editRoute(null);
                  }}
                >
                  <RotateCcw size={13} />
                  {td.resetLayout}
                </Button>
              </>
            }
            inspector={{
              open: inspectorOpen,
              onToggle: toggleInspector,
              title: td.inspectorTitle,
              showLabel: td.showInspector,
              hideLabel: td.hideInspector,
              content: (
                <DiagramInspector
                  design={shown.design}
                  model={shown.model}
                  selected={selected}
                  onSelect={setSelected}
                />
              ),
            }}
          >
            <DiagramCanvas
              nodes={flow.nodes}
              edges={flow.edges}
              selected={selected}
              onSelect={setSelected}
              onNodesChange={flow.onNodesChange}
              onEdgesChange={flow.onEdgesChange}
              onConnect={connect.onConnect}
              onEdgesDelete={connect.onEdgesDelete}
              onNodeDragStop={flow.onNodeDragStop}
            />
            <DiagramLegend categories={shown.model.legend} signal={layers.has("signal")} />
          </CanvasFrame>
        )}
      </QueryState>
      {connect.pending && (
        <NewNetDialog
          view={layers.has("power") && !layers.has("signal") ? "power" : "signal"}
          a={connect.pending.a}
          b={connect.pending.b}
          onClose={connect.clearPending}
        />
      )}
    </>
  );
}
