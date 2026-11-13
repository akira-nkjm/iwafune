import { type ReactNode, useId, useState } from "react";
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
import { Scene3D, type SceneEdit, type TransformMode } from "./Scene3D";
import { axesOf, boxesOf, DEFAULT_STRUCTURE_OPACITY, type V3 } from "./scene";

/** 移動のスナップ（選択肢の値は名前。刻み [m] は表から引く。選んだ文字列を数として読まない） */
const SNAPS = [
  { value: "1mm", label: "1 mm", step: 0.001 },
  { value: "5mm", label: "5 mm", step: 0.005 },
  { value: "10mm", label: "10 mm", step: 0.01 },
] as const;
const DEFAULT_SNAP = SNAPS[1];

/** 構造の配置の編集: 箱を選んで動かし、離すと Place を 1 回送る。重心の点も出す */
interface PlacementProps {
  variant: "placement";
  design: DesignView;
  /** 重心（機体座標、m。質量特性の出力が無ければ null） */
  com: V3 | null;
}

/** 熱の温度の色の表示: 箱を温度の色で塗る（3D の上では動かさない） */
interface HeatProps {
  variant: "heat";
  design: DesignView;
  /** 部品の箱の色（その部品の温度の色。`lib/tone.ts` の `heatHex`） */
  colorOf: (id: string) => string;
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
function PlacementLayout3D({ design, com }: PlacementProps) {
  const L = useT().systems.layout3d;
  const place = usePlaceBody();
  const [mode, setMode] = useState<TransformMode>("translate");
  const [snap, setSnap] = useState<string>(DEFAULT_SNAP.value);
  const step = (SNAPS.find((s) => s.value === snap) ?? DEFAULT_SNAP).step;
  return (
    <Layout3DFrame
      design={design}
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
            position: { x: p[0], y: p[1], z: p[2], unit: "m" },
            orientation: { w: q[0], x: q[1], y: q[2], z: q[3] },
            summary: L.moved(id),
          }),
      }}
    />
  );
}

/** 熱: 温度の色で塗った箱と、その凡例 */
function HeatLayout3D({ design, colorOf }: HeatProps) {
  const t = useT().systems;
  return (
    <Layout3DFrame
      design={design}
      title={t.thermal3d}
      note={` · ${t.thermal3dLegend}`}
      colorOf={colorOf}
    />
  );
}

/** 2 つの用途で共通の中身: 見出しの帯・3D の場面・右のインスペクタ（選んだ部品の配置と未配置の部品） */
function Layout3DFrame({
  design,
  title,
  note,
  colorOf,
  com = null,
  tools,
  edit,
  fill = false,
}: {
  design: DesignView;
  title?: string;
  /** 部品の数の後ろに添える補足（凡例・重心の印） */
  note: string;
  /** 箱の色（無ければ系の色） */
  colorOf?: (id: string) => string;
  com?: V3 | null;
  /** 見出しの右端の操作と、箱を動かす編集（配置の編集だけ） */
  tools?: ReactNode;
  edit?: SceneEdit;
  /** 配置ページでは残りの画面高を使う。熱の追加表示は既存の高さ */
  fill?: boolean;
}) {
  const L = useT().systems.layout3d;
  const [selected, setSelected] = useState<string | null>(null);
  const [showAxes, setShowAxes] = useState(true);
  const opacityId = useId();
  const [structureOpacity, setStructureOpacity] = useState(DEFAULT_STRUCTURE_OPACITY);
  const { boxes, unplaced, dimensions } = boxesOf(design, colorOf);
  const part = design.parts.find((p) => p.id === selected) ?? null;
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
          "grid min-w-0 grid-cols-1 grid-rows-[minmax(240px,1fr)_auto] overflow-y-auto @min-[700px]:grid-cols-[minmax(0,1fr)_minmax(0,340px)] @min-[700px]:grid-rows-1",
          fill ? "min-h-0 flex-1" : "h-[460px]",
        )}
      >
        <div className="relative min-h-[240px] min-w-0">
          {boxes.length === 0 ? (
            <Empty className="m-auto">{L.empty}</Empty>
          ) : (
            <Scene3D
              boxes={boxes}
              dimensions={dimensions}
              selected={selected}
              onSelect={setSelected}
              com={com}
              axes={showAxes ? axesOf(design) : null}
              edit={edit}
              structureOpacity={fill ? structureOpacity : undefined}
            />
          )}
          {showAxes && (
            <p className="pointer-events-none absolute top-3 left-3 max-w-[calc(100%-1.5rem)] rounded-control bg-panel/90 px-2 py-1 text-ink-3 text-xs">
              {L.axesLegend}
            </p>
          )}
        </div>
        <aside className="min-w-0 overflow-y-auto border-line border-t p-3 wrap-anywhere @min-[700px]:border-t-0 @min-[700px]:border-l">
          {part ? (
            <DetailSection title={L.placementOf(part.id)}>
              <PlacementSection
                key={`${part.id}-${design.project.revision}`}
                design={design}
                part={part}
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
                        <Chip onSelect={() => setSelected(id)}>{id}</Chip>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}
        </aside>
      </div>
    </Panel>
  );
}
