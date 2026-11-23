import {
  BaseEdge,
  type Edge,
  EdgeLabelRenderer,
  type EdgeProps,
  type Position,
  useReactFlow,
} from "@xyflow/react";
import { useEffect, useState } from "react";
import type { ViewEdge, ViewPortSide } from "@/api/types";
import { uiVar } from "@/lib/tone";
import { EdgeEditor } from "../editing/EdgeEditor";
import { RouteControls } from "../editing/RouteControls";
import { useViewEdits } from "../editing/view-edits";
import type { Pt } from "../layout/geometry";
import { TITLE_H } from "../layout/geometry";
import { placeLabels } from "../layout/labels";
import { followEnds, orthogonal, storedRoute } from "../layout/routes";
import { jumpPath, labelPoint } from "./jumps";
import { useOtherLines } from "./line-store";

export type RoutedEdgeType = Edge<
  {
    routeKey: string;
    /** 線のネット（同じネットの線どうしは交差に飛び越しを入れない） */
    nets: string[];
    manual?: ViewEdge;
    points: Pt[] | null;
    color: string;
    dashed: boolean;
    label: string | null;
    sourceLabel: string | null;
    targetLabel: string | null;
    selected: boolean;
    labelAt?: Pt;
  },
  "routed"
>;

type Props = EdgeProps<RoutedEdgeType>;

/** 自動配置の経路の端と、画面上の端子の位置のずれの許容 [px] */
const END_TOLERANCE = 2;

/** 線のラベルの重なり順（React Flow のノードより上） */
const LABEL_Z = 1000;

/** 線の端の向き（React Flow の Position は left / right / top / bottom の文字列） */
const sideOf = (p: Position): ViewPortSide => p;

function geometry(props: Props, preview: Pt[] | null) {
  const { data, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition } = props;
  const source = { x: sourceX, y: sourceY };
  const target = { x: targetX, y: targetY };
  const manual = data?.manual?.points.map(([x, y]) => ({ x, y }));
  // 自動配置の経路の両端が、画面上の端子の位置と合っているか。自動配置の座標と、DOM から測った端子の位置は
  // 小数の端数でわずかにずれるので、完全一致ではなく END_TOLERANCE までのずれを同じとみなす
  const near = (p: Pt | undefined, x: number, y: number) =>
    !!p && Math.abs(p.x - x) <= END_TOLERANCE && Math.abs(p.y - y) <= END_TOLERANCE;
  const fresh =
    near(data?.points?.[0], sourceX, sourceY) && near(data?.points?.at(-1), targetX, targetY);
  // 手で引いた経路は、端のカードを動かしていたら端をいまの端子につなぎ直す（followEnds）。
  // 自動配置の経路は、端が合っていればそのまま描き（端数のずれで 1 px の段ができないように）、
  // 合わなければ（カードを動かしたら）段差の線に戻す
  const points =
    preview ??
    (manual
      ? followEnds(
          manual,
          { at: source, side: sideOf(sourcePosition) },
          { at: target, side: sideOf(targetPosition) },
        )
      : fresh && data?.points
        ? data.points
        : orthogonal([
            source,
            ...(data?.points?.slice(1, -1) ?? [
              { x: (sourceX + targetX) / 2, y: sourceY },
              { x: (sourceX + targetX) / 2, y: targetY },
            ]),
            target,
          ]));
  return { points, fresh };
}

const NO_NETS: string[] = [];

function useLineLabel(props: Props, points: Pt[], fresh: boolean, editing: boolean): Pt {
  const { getNodes, getInternalNode, getEdges } = useReactFlow();
  if (!editing && fresh && props.data?.labelAt) return props.data.labelAt;
  const routes = new Map(
    getEdges().flatMap((e) => (e.data?.points ? [[e.id, e.data.points as Pt[]] as const] : [])),
  );
  routes.set(props.id, points);
  // 避けるもの: カードと、枠の見出しの帯（自動配置のラベルと同じ。layout.ts）
  const obstacles = getNodes()
    .filter((n) => n.type === "card" || n.type === "container")
    .flatMap((n) => {
      const internal = getInternalNode(n.id);
      return internal
        ? [
            {
              ...internal.internals.positionAbsolute,
              width: n.measured?.width ?? 150,
              height: n.type === "container" ? TITLE_H : (n.measured?.height ?? 52),
            },
          ]
        : [];
    });
  return (
    placeLabels(routes, new Map([[props.id, props.data?.label ?? ""]]), obstacles).get(props.id) ??
    labelPoint(points)
  );
}

/**
 * 端子の名前を置く向き。カードの外の、線の脇: 上下の側は線の右（上の側は上へ、下の側は下へ）、左右の側は線の上
 * （左の側は左へ、右の側は右へ）。端子の真上に重ねるとカードの縁や隣の端子の名前と重なるため。
 */
const END_LABEL: Record<ViewPortSide, string> = {
  top: "translate(4px, calc(-100% - 2px))",
  bottom: "translate(4px, 2px)",
  left: "translate(calc(-100% - 4px), calc(-100% - 1px))",
  right: "translate(4px, calc(-100% - 1px))",
};

function LineLabels({
  data,
  mid,
  start,
  finish,
  startSide,
  finishSide,
}: {
  data: Props["data"];
  mid: Pt;
  start: Pt;
  finish: Pt;
  startSide: ViewPortSide;
  finishSide: ViewPortSide;
}) {
  const labels = [
    { key: "protocol", label: data?.label, at: mid, shift: "translate(-50%, -50%)", main: true },
    {
      key: "source",
      label: data?.sourceLabel,
      at: start,
      shift: END_LABEL[startSide],
      main: false,
    },
    {
      key: "target",
      label: data?.targetLabel,
      at: finish,
      shift: END_LABEL[finishSide],
      main: false,
    },
  ];
  return (
    <>
      {labels.map(
        ({ key, label, at, shift, main }) =>
          label && (
            <div
              key={key}
              className={
                main
                  ? "nodrag nopan pointer-events-none absolute rounded-[5px] bg-panel/95 px-1 font-bold text-[10px] text-ink"
                  : "pointer-events-none absolute whitespace-nowrap rounded-[4px] bg-panel/90 px-0.5 font-mono text-[9.5px] text-ink-3 leading-none"
              }
              style={{
                transform: `translate(${at.x}px, ${at.y}px) ${shift}`,
                // 枠（サブシステムの箱）やカードの上に描く。下に描くと枠の地に隠れて読めない
                zIndex: LABEL_Z,
              }}
            >
              {label}
            </div>
          ),
      )}
    </>
  );
}

/**
 * 系統図の線。自動配置の経路か手で引いた経路を描き、ほかのネットの線と交わる所に飛び越しを入れる（水平の線が
 * 垂直の線をまたぐ。相手の経路は line-store.ts で分け合うので、手で引いた線や動かした線にも入る）。選ぶと取っ手
 * （EdgeEditor）と操作のボタン（RouteControls: 自動に戻す・端子の側を変える）を出す。端子の側は表示だけで、
 * つながりは変えない。
 */
export function RoutedEdge(props: Props) {
  const { data, sourceX, sourceY, targetX, targetY } = props;
  const { editRoute: edit } = useViewEdits();
  const [preview, setPreview] = useState<Pt[] | null>(null);
  // 保存した経路は、読み直した設計に入るまで描き続ける（離した瞬間に元の経路へ戻らないように）
  const [pending, setPending] = useState<Pt[] | null>(null);
  // biome-ignore lint/correctness/useExhaustiveDependencies: 設計を読み直して手引きの経路が変わったら、保存待ちを外す
  useEffect(() => setPending(null), [data?.manual]);
  const shown = preview ?? pending;
  const { points, fresh } = geometry(props, shown);
  const others = useOtherLines(props.id, points, data?.nets ?? NO_NETS);
  const path = jumpPath(points, others);
  const mid = useLineLabel(props, points, fresh, !!data?.manual || !!shown);
  const save = (pts: Pt[], sides = data?.manual) => {
    if (!data) return;
    const route = storedRoute(pts, sides);
    setPending(route.points.map(([x, y]) => ({ x, y })));
    edit(data.routeKey, route, () => setPending(null));
  };
  const start = { x: sourceX, y: sourceY };
  const finish = { x: targetX, y: targetY };
  return (
    <>
      <BaseEdge
        id={props.id}
        path={path}
        interactionWidth={14}
        style={{
          stroke: data?.color ?? uiVar.ink3,
          strokeWidth: data?.selected ? 3 : 1.6,
          strokeDasharray: data?.dashed ? "5 3" : undefined,
        }}
      />
      <EdgeLabelRenderer>
        {data?.selected && (
          <>
            <EdgeEditor points={points} onChange={save} onPreview={setPreview} />
            <RouteControls
              at={mid}
              manual={data.manual}
              source={props.source}
              target={props.target}
              start={start}
              finish={finish}
              points={points}
              onPreview={setPreview}
              onReset={() => edit(data.routeKey)}
              // 端を縁の好きな位置へ付け替えた: 側と位置（左か上の角からの距離）を経路と一緒に保存する
              onAttach={(end, { side, offset, points: next }) =>
                save(next, {
                  ...storedRoute(points, data.manual),
                  [`${end}_side`]: side,
                  [`${end}_offset`]: offset,
                })
              }
              // 側だけを選び直した: 位置は端子の並びで決まる位置に戻す
              onSide={(end, side) =>
                save(points, {
                  ...storedRoute(points, data.manual),
                  [`${end}_side`]: side,
                  [`${end}_offset`]: undefined,
                })
              }
            />
          </>
        )}
        <LineLabels
          data={data}
          mid={mid}
          start={start}
          finish={finish}
          startSide={sideOf(props.sourcePosition)}
          finishSide={sideOf(props.targetPosition)}
        />
      </EdgeLabelRenderer>
    </>
  );
}
