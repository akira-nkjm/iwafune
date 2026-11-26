import { LayoutGrid, RotateCcw } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router";
import { onBeforeHistoryStep } from "@/api/history";
import { useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { SkeletonCanvas } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Refreshable } from "@/kit/feedback/Refreshable";
import { UndoRedoButtons } from "@/kit/UndoRedoButtons";
import { afterPaint } from "@/lib/after-paint";
import { diagramHiddenSystems, withDiagramHiddenSystems } from "@/lib/nav";
import { CanvasFrame } from "@/shell/CanvasFrame";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import { DiagramCanvas } from "./canvas/DiagramCanvas";
import { useDiagramFlow, useDiagramModel } from "./canvas/use-diagram-flow";
import { DiagramInspector, type DiagramSelection } from "./DiagramInspector";
import { DiagramLegend } from "./DiagramLegend";
import { DiagramSystems } from "./DiagramSystems";
import { DiagramToolbar } from "./DiagramToolbar";
import { NewNetDialog } from "./editing/NewNetDialog";
import { useDiagramConnect } from "./editing/use-diagram-connect";
import { useViewEdits } from "./editing/view-edits";
import type { Layer } from "./power";

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
  const [params, setParams] = useSearchParams();
  const hiddenSystems = useMemo(
    () => diagramHiddenSystems(params, d?.defs ?? { systems: [], diagram_hidden_systems: [] }),
    [params, d?.defs],
  );
  const {
    model,
    layout,
    pending,
    error: layoutError,
    retry,
  } = useDiagramModel(d, layers, merge, hiddenSystems);
  // 表示の切り替え（系・層・まとめ方）は図を作り直す重い更新なので、先に「作り直し中」を画面に出してから始める。
  // そうしないと、押しても作り直しが終わるまで何も変わらず、固まって見える
  const [queued, setQueued] = useState(false);
  const later = (apply: () => void) => {
    setQueued(true);
    afterPaint(apply);
  };
  // 作り直しが済んだら（図が変わらない切り替えなら、すぐ）戻す
  // biome-ignore lint/correctness/useExhaustiveDependencies: 切り替えた値が変わったときにも確かめ直す
  useEffect(() => {
    if (!pending) setQueued(false);
  }, [pending, hiddenSystems, layers, merge]);
  const flow = useDiagramFlow(d, model, layout);
  // 元に戻す・やり直すの前に、手で動かした印と保存待ちの位置を捨てる（保存した位置だけを描くように）。
  // キー（⌘Z・⇧⌘Z。画面全体で受ける。shell/use-history-keys.ts）でもボタンでも同じ
  const { resetMoved } = flow;
  useEffect(() => onBeforeHistoryStep(resetMoved), [resetMoved]);
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
      {/* 設計から作った図（配置は空き時間に計算する）が最初にできるまでは、図のスケルトン。表示を切り替えて位置を
          探し直す間は、前の図を薄くして残し、上端に読み込み中の帯を出す（useDiagramModel の pending） */}
      <QueryState
        query={{
          data: d && model && layout && !layoutError ? { design: d, model } : undefined,
          error: design.error ?? layoutError,
          refetch: design.error ? design.refetch : retry,
        }}
        failure={design.error ? undefined : td.cannotLayOut}
        className="flex min-h-0 flex-1 flex-col"
        skeleton={<SkeletonCanvas label={t.common.working.layingOutDiagram} />}
      >
        {(shown) => (
          <CanvasFrame
            toolbar={
              <>
                <DiagramSystems
                  defs={shown.design.defs}
                  hidden={hiddenSystems}
                  onChange={(hidden) => {
                    setSelected(null);
                    later(() =>
                      setParams((previous) => withDiagramHiddenSystems(previous, hidden)),
                    );
                  }}
                />
                <DiagramToolbar
                  layers={layers}
                  onLayers={(next) => later(() => setLayers(next))}
                  merge={merge}
                  onMerge={(next) => later(() => setMerge(next))}
                />
                <UndoRedoButtons />
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
            <Refreshable
              active={pending || queued}
              blockInput
              immediate
              label={t.common.working.layingOutDiagram}
              className="absolute inset-0"
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
            </Refreshable>
          </CanvasFrame>
        )}
      </QueryState>
      {connect.pending && (
        <NewNetDialog
          view={layers.has("power") && !layers.has("signal") ? "power" : "signal"}
          a={connect.pending.a}
          b={connect.pending.b}
          open
          onOpenChange={(open) => !open && connect.clearPending()}
        />
      )}
    </>
  );
}
