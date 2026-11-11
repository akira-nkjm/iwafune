import { type Edge, type Node, useEdgesState, useNodesState } from "@xyflow/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { DesignView, ViewNodePos, ViewPortSide } from "@/api/types";
import { getT, useLang } from "@/i18n";
import { savedLayout, snap, viewId } from "../editing/saved";
import { useViewEdits, VIEW } from "../editing/view-history";
import type { DiagramLayout } from "../layout/geometry";
import { drawLayout, type LayoutPlan, planLayout } from "../layout/layout";
import { buildModel, type DiagramModel } from "../model";
import { LAYERS, type Layer } from "../power";
import type { Attachment } from "./CardNode";
import { flowEdges, flowNodes } from "./flow";

/**
 * React Flow のハンドル ID。手で側を選んだ端は `ポート@側`、位置まで決めた端は `ポート@側@位置`
 * （カードはその位置に端子を描く。CardNode）。手で決めていなければポートの ID のまま。
 */
function handleOf(
  base: string | null | undefined,
  side: ViewPortSide | null | undefined,
  offset: number | null | undefined,
): string | null | undefined {
  if (!base || !side) return base;
  return offset == null ? `${base}@${side}` : `${base}@${side}@${offset}`;
}

/** 位置を決めるときに使う層（全部）。表示する層を切り替えてもカードの位置が変わらないように */
const ALL_LAYERS: ReadonlySet<Layer> = new Set(LAYERS);

/** 中身が同じ表示モデルは同じものとして扱う（設計を読み直すたびに新しいオブジェクトになるため） */
function useStableModel(built: DiagramModel | null): DiagramModel | null {
  const key = useMemo(() => (built ? JSON.stringify(built) : ""), [built]);
  // biome-ignore lint/correctness/useExhaustiveDependencies: built の中身は key で見る
  return useMemo(() => built, [key]);
}

/**
 * 表示モデルと自動配置。位置（枠・カード・端子）は全部の層の線を入れた表示モデルで決め（planLayout）、表示する
 * 層の線だけをその上に引く（drawLayout）。層を切り替えても位置は変わらず、線を引き直すだけ。位置の探索は軽くないので
 * 描画のあとに回し、終わるまでは layout が null（画面はスケルトン）。位置や線の経路を保存して設計を読み直しても、
 * 表示モデルの中身が同じなら探索し直さない。
 */
export function useDiagramModel(
  d: DesignView | undefined,
  layers: ReadonlySet<Layer>,
  merge: boolean,
) {
  const lang = useLang();
  const model = useStableModel(
    useMemo(
      () => (d ? buildModel(d, { layers, mergeSiblings: merge, lang }) : null),
      [d, layers, merge, lang],
    ),
  );
  const full = useStableModel(
    useMemo(
      () => (d ? buildModel(d, { layers: ALL_LAYERS, mergeSiblings: merge, lang }) : null),
      [d, merge, lang],
    ),
  );
  const [plan, setPlan] = useState<{ full: DiagramModel; plan: LayoutPlan } | null>(null);
  useEffect(() => {
    if (!full) return;
    const timer = setTimeout(() => setPlan({ full, plan: planLayout(full) }), 0);
    return () => clearTimeout(timer);
  }, [full]);
  const layout = useMemo(
    () => (plan && model && plan.full === full ? drawLayout(plan.plan, model) : null),
    [plan, model, full],
  );
  return { model, layout };
}

/**
 * 位置と枠の大きさを view/electrical.toml に保存する（set_view_layout。元に戻せる。view-history.ts）。
 * `remove` に挙げたノードの位置は消す（自動配置の位置に戻る）。失敗したら `onError` を呼ぶ
 */
export function usePersistLayout() {
  const { send } = useViewEdits();
  return useCallback(
    (nodes: ViewNodePos[], summary: string, onError?: () => void, remove: string[] = []) =>
      send([{ op: "set_view_layout", view: VIEW, nodes, remove }], summary, onError),
    [send],
  );
}

/**
 * React Flow のノードと辺。保存した位置があればそれを使い、手で動かしたノードは記録して辺を段差の線に戻す。
 * 自動配置の再実行後も、保存した位置・経路を常に優先する。
 */
export function useDiagramFlow(
  d: DesignView | undefined,
  model: DiagramModel | null,
  layout: DiagramLayout | null,
) {
  const persist = usePersistLayout();
  const [moved, setMoved] = useState<ReadonlySet<string>>(new Set());
  const stored = useMemo(() => (d ? savedLayout(d, VIEW) : new Map<string, ViewNodePos>()), [d]);
  // 保存待ちの位置（動かして離したが、まだサーバーの返事が来ていないもの）。返事が来るまでこちらを描く。
  // 描き直しに保存前の位置を使うと、離した瞬間に一度元の位置へ戻ってしまうため
  const [pending, setPending] = useState<ReadonlyMap<string, ViewNodePos>>(new Map());
  const saved = useMemo(() => new Map([...stored, ...pending]), [stored, pending]);
  // 読み直した設計に同じ位置が入ったら、保存待ちから外す
  useEffect(() => {
    setPending((p) => {
      const left = [...p].filter(([id, pos]) => {
        const now = stored.get(id);
        return !now || now.x !== pos.x || now.y !== pos.y;
      });
      return left.length === p.size ? p : new Map(left);
    });
  }, [stored]);
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  // nodes を依存にすると、ノードを置き直すたびにコールバックが変わって描き直しが止まらなくなる
  const nodesRef = useRef(nodes);
  nodesRef.current = nodes;
  const onResized = useCallback(
    (id: string, w: number, h: number) => {
      const n = nodesRef.current.find((x) => x.id === id);
      if (!n) return;
      const pos = {
        id: viewId(id),
        x: snap(n.position.x),
        y: snap(n.position.y),
        w: snap(w),
        h: snap(h),
      };
      persist([pos], getT().diagram.resizeSummary(viewId(id)));
    },
    [persist],
  );
  useEffect(() => {
    if (!model || !layout) return;
    const view = d?.views.find((v) => v.id === VIEW);
    // 手で決めた端の付け所（側と位置）。カードにその位置の端子を描かせる
    const attachments = new Map<string, Attachment[]>();
    const attach = (node: string, id: string, side: ViewPortSide, offset: number) =>
      attachments.set(node, [...(attachments.get(node) ?? []), { id, side, offset }]);
    const edges = flowEdges(model, layout, moved, saved).map((edge) => {
      const manual = view?.edge?.[String(edge.data?.routeKey)];
      const tap = layout.taps.get(edge.id);
      // 線の端の付け所: 手で決めた側と位置があればそれ、無ければ自動配置の分かれ口、どちらも無ければ端子
      const handle = (
        which: "source" | "target",
        base: string | null | undefined,
        node: string,
      ) => {
        const side = manual?.[`${which}_side`];
        const offset = manual?.[`${which}_offset`];
        if (side) {
          const h = handleOf(base, side, offset);
          if (h && offset != null) attach(node, h, side, offset);
          return h;
        }
        const t = tap?.[which];
        if (!t) return base;
        const along = t.side === "left" || t.side === "right" ? t.y : t.x;
        const h = handleOf(base, t.side, along);
        if (h) attach(node, h, t.side, along);
        return h;
      };
      const sourceHandle = handle("source", edge.sourceHandle, edge.source);
      const targetHandle = handle("target", edge.targetHandle, edge.target);
      return { ...edge, sourceHandle, targetHandle, data: { ...edge.data, manual } };
    });
    setNodes(
      flowNodes(model, layout, saved, { onResized }).map((n) =>
        attachments.has(n.id)
          ? { ...n, data: { ...n.data, attachments: attachments.get(n.id) } }
          : n,
      ),
    );
    setEdges(edges);
  }, [d, model, layout, saved, moved, onResized, setNodes, setEdges]);
  const onNodeDragStop = useCallback(
    (_: unknown, node: Node, dragged: Node[]) => {
      const list = (dragged.length > 0 ? dragged : [node]).filter((n) => n.type !== "block");
      setMoved((m) => new Set([...m, ...list.map((n) => n.id)]));
      const pos = list.map((n) => ({
        id: viewId(n.id),
        x: snap(n.position.x),
        y: snap(n.position.y),
        w: n.width ? snap(n.width) : null,
        h: n.height ? snap(n.height) : null,
      }));
      setPending((p) => new Map([...p, ...pos.map((x) => [x.id, x] as const)]));
      // 保存できなかったら、保存待ちの位置を取り消して元の位置に戻す
      const undo = () =>
        setPending((p) => new Map([...p].filter(([id]) => !pos.some((x) => x.id === id))));
      persist(pos, getT().diagram.moveSummary(list.map((n) => viewId(n.id)).join(", ")), undo);
    },
    [persist],
  );
  // 手で動かした印と保存待ちの位置を捨てる（配置を戻す・元に戻す・やり直す前に。保存した位置だけを描くように）
  const resetMoved = useCallback(() => {
    setMoved(new Set());
    setPending(new Map());
  }, []);
  // 自動配置: 手で動かした枠とカードの位置を消して、自動配置の位置に戻す（手で引いた線の経路は残す。端はカードに付いていく）
  const autoLayout = useCallback(() => {
    setMoved(new Set());
    setPending(new Map());
    const ids = [...stored.keys()];
    if (ids.length > 0) persist([], getT().diagram.autoLayoutSummary, undefined, ids);
  }, [stored, persist]);
  return { nodes, edges, onNodesChange, onEdgesChange, onNodeDragStop, resetMoved, autoLayout };
}
