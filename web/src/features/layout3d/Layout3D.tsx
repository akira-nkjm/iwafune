import { type ReactNode, useId, useMemo, useReducer, useState } from "react";
import { usePlaceBody } from "@/api/ops";
import type { DesignView } from "@/api/types";
import { PlacementSection } from "@/features/components/detail/PlacementSection";
import { useT } from "@/i18n";
import { DetailSection } from "@/kit/detail/DetailSection";
import { Empty } from "@/kit/feedback/Empty";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { RangeInput } from "@/kit/forms/RangeInput";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { Chip } from "@/kit/selection/Chip";
import { Segmented } from "@/kit/selection/Segmented";
import { Switch } from "@/kit/selection/Switch";
import { cn } from "@/lib/cn";
import { useScheme } from "@/lib/theme";
import { HelperToggles } from "./HelperToggles";
import { LayoutBrowser } from "./LayoutBrowser";
import { Scene3D, type SceneEdit, type TransformMode } from "./Scene3D";
import {
  axesOf,
  boxesOf,
  DEFAULT_STRUCTURE_OPACITY,
  fovsOf,
  hasSize,
  type SunArrow,
  type V3,
} from "./scene";
import { ViewControls3D } from "./ViewControls3D";
import type { ZoomRequest } from "./ViewInput3D";
import {
  DEFAULT_SCENE_HELPERS,
  type FocusRequest,
  HEAT_SCENE_HELPERS,
  type SceneHelpers,
} from "./visibility";

/** 移動のスナップ（選択肢の値は名前。刻み [m] は表から引く。選んだ文字列を数として読まない） */
const SNAPS = [
  { value: "1mm", label: "1 mm", step: 0.001 },
  { value: "5mm", label: "5 mm", step: 0.005 },
  { value: "10mm", label: "10 mm", step: 0.01 },
] as const;
const DEFAULT_SNAP = SNAPS[1];
/** 最初は隠した部品が無い */
const NONE_HIDDEN: ReadonlySet<string> = new Set();

/** 構造の配置の編集: 箱を選んで動かし、離すと Place を 1 回送る。重心の点も出す */
interface PlacementProps {
  variant: "placement";
  design: DesignView;
  configuration?: string | null;
  /** 重心（機体座標、m。質量特性の出力が無ければ null） */
  com: V3 | null;
}

/** 熱の温度の色の表示: 箱を温度の色で塗る（3D の上では動かさない） */
interface HeatProps {
  variant: "heat";
  design: DesignView;
  /** 部品の箱の色（その部品の温度の色。`lib/tone.ts` の `heatHex`） */
  colorOf: (id: string) => string;
  /** 見出しの帯に置く操作（温度の出どころの選択など） */
  tools?: ReactNode;
  /** 3D の上（左下）に重ねるもの（色の凡例） */
  overlay?: ReactNode;
  /** 3D の右上（視点を戻すボタンの下）に重ねるもの（軌道の図）。`sun` は太陽の矢印（非定常のときだけ。無ければ矢印もボタンも出さない） */
  corner?: ReactNode;
  sun?: SunArrow | null;
  /** 3D の右（狭い幅では下）に置くもの（節点の温度の一覧）。配置の欄の代わりに出す */
  aside?: ReactNode;
}

/** 3D 配置の用途（`variant`）ごとの Props。用途に無い組み合わせ（熱の色で動かすなど）は型で作れない */
export type Layout3DProps = PlacementProps | HeatProps;

/**
 * 3D 配置の表示。構造の配置の編集（`placement`。15° と 1/5/10 mm でスナップ）と、熱の温度の色の表示（`heat`）。
 * どちらも右のインスペクタで、選んだ部品の位置・姿勢を数値で入れられる（同じ Op）。
 */
export function Layout3D(props: Layout3DProps) {
  return props.variant === "placement" ? (
    <PlacementLayout3D {...props} />
  ) : (
    <HeatLayout3D {...props} />
  );
}

/** 構造: 移動・回転とスナップを選び、箱を離したときに Place を送る */
function PlacementLayout3D({ design, com, configuration }: PlacementProps) {
  const L = useT().systems.layout3d;
  const place = usePlaceBody();
  const [mode, setMode] = useState<TransformMode>("translate");
  const [snap, setSnap] = useState<string>(DEFAULT_SNAP.value);
  const step = (SNAPS.find((s) => s.value === snap) ?? DEFAULT_SNAP).step;
  return (
    <Layout3DFrame
      design={design}
      configuration={configuration}
      note={com ? L.com : ""}
      com={com}
      fill
      tools={
        <div className="ml-auto flex items-center gap-2">
          <Segmented
            size="sm"
            label={L.operation}
            value={mode}
            onChange={setMode}
            options={[
              { value: "translate", label: L.translate },
              { value: "rotate", label: L.rotate },
            ]}
          />
          <NativeSelect
            size="sm"
            aria-label={L.snap}
            value={snap}
            onChange={(e) => setSnap(e.target.value)}
            options={SNAPS}
          />
        </div>
      }
      edit={{
        mode,
        snap: step,
        onPlace: (id, p, q) =>
          // 機体座標のまま送る。取付点に対する相対への換算はサーバー（POST /parts/{id}/place_body）
          place.mutate({
            part: id,
            configuration: configuration ?? undefined,
            position: { x: p[0], y: p[1], z: p[2], unit: "m" },
            orientation: { w: q[0], x: q[1], y: q[2], z: q[3] },
            summary: L.moved(id),
          }),
      }}
    />
  );
}

/** 熱: 温度の色で塗った箱。3D を大きく見せるため、画面の高さに合わせ、右は節点の一覧（`aside`）だけにする */
function HeatLayout3D({ design, colorOf, tools, overlay, corner, sun, aside }: HeatProps) {
  const t = useT().systems;
  return (
    <Layout3DFrame
      design={design}
      note={` · ${t.thermal3dLegend}`}
      colorOf={colorOf}
      tools={tools}
      overlay={overlay}
      corner={corner}
      sun={sun}
      aside={aside}
      initialHelpers={HEAT_SCENE_HELPERS}
    />
  );
}

/** 2 つの用途で共通の中身: 見出しの帯・（配置のページでは左の部品のツリー）・3D の場面・右のインスペクタ */
function Layout3DFrame({
  design,
  configuration,
  title,
  note,
  colorOf,
  com = null,
  tools,
  edit,
  fill = false,
  initialHelpers = DEFAULT_SCENE_HELPERS,
  overlay,
  corner,
  sun,
  aside,
}: {
  design: DesignView;
  configuration?: string | null;
  title?: string;
  /** 部品の数の後ろに添える補足（凡例・重心の印） */
  note: string;
  /** 箱の色（無ければ系の色） */
  colorOf?: (id: string) => string;
  com?: V3 | null;
  /** 見出しの右端の操作と、箱を動かす編集（配置の編集だけ） */
  tools?: ReactNode;
  edit?: SceneEdit;
  /** 配置ページ: 残りの画面高を使い、部品のツリー・表示の切り替え・構造の不透明度を出す。熱の追加表示は既存の高さ */
  fill?: boolean;
  /** 補助の表示（視野・軸・格子）の最初の状態 */
  initialHelpers?: SceneHelpers;
  /** 3D の左下に重ねるもの（熱の色の凡例） */
  overlay?: ReactNode;
  /** 3D の右上（視点を戻すボタンの下）に重ねるもの（軌道の図）と、太陽の矢印（熱の非定常） */
  corner?: ReactNode;
  sun?: SunArrow | null;
  /** インスペクタの代わりに右へ置くもの（熱の節点の一覧） */
  aside?: ReactNode;
}) {
  const L = useT().systems.layout3d;
  const [selected, setSelected] = useState<string | null>(null);
  const [showAxes, setShowAxes] = useState(false);
  const opacityId = useId();
  const scheme = useScheme();
  const [structureOpacity, setStructureOpacity] = useState(DEFAULT_STRUCTURE_OPACITY);
  const [hidden, setHidden] = useState<ReadonlySet<string>>(NONE_HIDDEN);
  const [helpers, setHelpers] = useState<SceneHelpers>(initialHelpers);
  const [focus, setFocus] = useState<FocusRequest | undefined>(undefined);
  const [home, setHome] = useState(0);
  // ボタンの拡大・縮小の依頼（押すたびに n が増える）
  const [zoom, zoomBy] = useReducer(
    (old: ZoomRequest | null, factor: number) => ({ n: (old?.n ?? 0) + 1, factor }),
    null,
  );
  const { boxes, unplaced, dimensions } = useMemo(
    () => boxesOf(design, colorOf, scheme),
    [design, colorOf, scheme],
  );
  const placed = useMemo(() => new Set(boxes.map((b) => b.id)), [boxes]);
  const fovs = useMemo(() => fovsOf(design), [design]);
  const shown = boxes.filter((b) => !hidden.has(b.id));
  // ツリーで押した部品は選び、視点を寄せる（3D で押したときは選ぶだけ）
  const pick = (id: string) => {
    setSelected(id);
    setFocus((old) => ({ id, n: (old?.n ?? 0) + 1 }));
  };
  return (
    <Panel
      pad={false}
      className={cn("@container overflow-hidden", fill && "flex min-h-0 flex-1 flex-col")}
    >
      {title && (
        <PanelHeader
          title={title}
          caption={note}
          className="mb-0 shrink-0 flex-wrap border-line border-b px-4 py-2"
        />
      )}
      <div className="flex shrink-0 flex-wrap items-center gap-3 border-line border-b px-4 py-2">
        <p className="text-ink-3 text-xs">
          {L.parts(boxes.length)}
          {unplaced.length > 0 ? L.unplacedCount(unplaced.length) : ""}
          {!title && note}
        </p>
        <div className="ml-auto flex min-w-0 flex-wrap items-center gap-3">
          <Switch value={showAxes} onChange={setShowAxes}>
            {L.axes}
          </Switch>
          {/* 視野・原点の軸・格子・隅の軸の出し入れは、配置でも熱画像でも使える */}
          <HelperToggles value={helpers} onChange={setHelpers} hasSun={Boolean(sun)} />
          {fill && (
            <label
              htmlFor={opacityId}
              className="flex min-w-0 flex-wrap items-center gap-2 text-ink-2 text-xs"
            >
              {L.structureOpacity}
              <RangeInput
                id={opacityId}
                aria-label={L.structureOpacity}
                min={0}
                max={1}
                step={0.01}
                value={structureOpacity}
                onChange={setStructureOpacity}
                className="w-24 max-w-full"
              />
            </label>
          )}
          {tools}
        </div>
      </div>
      <div
        className={cn(
          "grid min-w-0 grid-cols-1 overflow-y-auto",
          fill
            ? "min-h-0 flex-1 grid-rows-[minmax(240px,1fr)_auto_auto] @min-[700px]:grid-cols-[minmax(0,240px)_minmax(0,1fr)] @min-[700px]:grid-rows-[minmax(240px,1fr)_auto] @min-[1100px]:grid-cols-[minmax(0,260px)_minmax(0,1fr)_minmax(0,320px)] @min-[1100px]:grid-rows-1"
            : "h-[calc(100dvh-15rem)] min-h-[460px] grid-rows-[minmax(320px,1fr)_auto] @min-[900px]:grid-cols-[minmax(0,1fr)_minmax(0,260px)] @min-[900px]:grid-rows-1",
        )}
      >
        {fill && (
          <aside
            aria-label={L.browser}
            className="order-2 h-72 min-h-0 min-w-0 overflow-hidden border-line border-t @min-[700px]:order-none @min-[700px]:h-auto @min-[700px]:border-t-0 @min-[700px]:border-r"
          >
            <LayoutBrowser
              design={design}
              placed={placed}
              selected={selected}
              onSelect={pick}
              hidden={hidden}
              onHidden={setHidden}
            />
          </aside>
        )}
        <div className="relative order-1 min-h-[240px] min-w-0 @min-[700px]:order-none">
          {boxes.length === 0 ? (
            <Empty className="m-auto">{L.empty}</Empty>
          ) : (
            <Scene3D
              key={configuration}
              boxes={shown}
              dimensions={dimensions}
              selected={selected}
              onSelect={setSelected}
              com={com}
              axes={showAxes ? axesOf(design) : null}
              edit={edit}
              structureOpacity={fill ? structureOpacity : undefined}
              helpers={helpers}
              focus={focus ?? null}
              home={home}
              zoom={zoom}
              fovs={fovs}
              sun={helpers.sun ? (sun ?? null) : null}
            />
          )}
          {boxes.length > 0 && (
            <ViewControls3D onZoom={zoomBy} onHome={() => setHome((n) => n + 1)} />
          )}
          {overlay && (
            <div className="pointer-events-none absolute bottom-3 left-3 max-w-[calc(100%-1.5rem)] rounded-control bg-panel/90 px-2 py-1">
              {overlay}
            </div>
          )}
          {corner && (
            <div className="pointer-events-none absolute top-14 right-3 max-w-[calc(100%-1.5rem)] rounded-control bg-panel/90 p-1">
              {corner}
            </div>
          )}
          {showAxes && (
            <p className="pointer-events-none absolute top-3 left-3 max-w-[calc(100%-1.5rem)] rounded-control bg-panel/90 px-2 py-1 text-ink-3 text-xs">
              {L.axesLegend}
            </p>
          )}
        </div>
        {aside ? (
          <aside className="order-3 min-w-0 overflow-y-auto border-line border-t p-3 @min-[900px]:order-none @min-[900px]:border-t-0 @min-[900px]:border-l">
            {aside}
          </aside>
        ) : (
          <Inspector
            design={design}
            configuration={configuration}
            selected={selected}
            onSelect={pick}
            unplaced={unplaced}
            className={cn(
              "order-3 @min-[700px]:order-none",
              fill
                ? "@min-[700px]:col-span-2 @min-[1100px]:col-span-1 @min-[1100px]:border-t-0 @min-[1100px]:border-l"
                : "@min-[700px]:border-t-0 @min-[700px]:border-l",
            )}
          />
        )}
      </div>
    </Panel>
  );
}

/** 右（狭い幅では下）のインスペクタ: 選んだ部品の配置の欄か、操作の案内と未配置の部品 */
function Inspector({
  design,
  configuration,
  selected,
  onSelect,
  unplaced,
  className,
}: {
  design: DesignView;
  configuration?: string | null;
  selected: string | null;
  onSelect: (id: string) => void;
  unplaced: string[];
  className?: string;
}) {
  const L = useT().systems.layout3d;
  const part = design.parts.find((p) => p.id === selected) ?? null;
  return (
    <aside
      className={cn("min-w-0 overflow-y-auto border-line border-t p-3 wrap-anywhere", className)}
    >
      {part ? (
        <DetailSection title={L.placementOf(part.id)}>
          {!hasSize(part) && <p className="mb-2 text-ink-3 text-xs">{L.noSize}</p>}
          <PlacementSection
            key={JSON.stringify([part.id, design.project.revision, configuration, part.mount])}
            design={design}
            part={part}
            configuration={configuration}
          />
        </DetailSection>
      ) : (
        <div className="space-y-2 text-xs">
          <p className="text-ink-3">{L.hint}</p>
          {unplaced.length > 0 && (
            <div>
              <p className="mb-1 font-medium text-ink-2">{L.unplaced}</p>
              <ul className="flex flex-wrap gap-1">
                {unplaced.map((id) => (
                  <li key={id}>
                    <Chip onSelect={() => onSelect(id)}>{id}</Chip>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}
    </aside>
  );
}
