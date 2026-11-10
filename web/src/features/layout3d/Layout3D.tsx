import { useState } from "react";
import { usePlaceBody } from "@/api/ops";
import type { DesignView } from "@/api/types";
import { PlacementSection } from "@/features/components/PlacementSection";
import { useT } from "@/i18n";
import { Chip } from "@/kit/Chip";
import { DetailSection } from "@/kit/DetailSection";
import { Empty } from "@/kit/Empty";
import { NativeSelect } from "@/kit/NativeSelect";
import { Panel } from "@/kit/Panel";
import { Segmented } from "@/kit/Segmented";
import { Switch } from "@/kit/Switch";
import { Scene3D, type TransformMode } from "./Scene3D";
import { axesOf, boxesOf, type V3 } from "./scene";

/** 移動のスナップ（選択肢の値は名前。刻み [m] は表から引く。選んだ文字列を数として読まない） */
const SNAPS = [
  { value: "1mm", label: "1 mm", step: 0.001 },
  { value: "5mm", label: "5 mm", step: 0.005 },
  { value: "10mm", label: "10 mm", step: 0.01 },
] as const;
const DEFAULT_SNAP = SNAPS[1];

/**
 * 3D 配置の表示・編集。箱を選んで動かし、離すと Place を 1 回送る（15° と 1/5/10 mm でスナップ）。
 * 右のインスペクタでは位置・姿勢を数値でも入れられる（同じ Op）。editable=false なら表示だけ（熱の温度の色など）。
 */
export function Layout3D({
  design,
  colorOf,
  com,
  editable = true,
  title,
  legend,
}: {
  design: DesignView;
  colorOf?: (id: string) => string | undefined;
  com?: V3 | null;
  editable?: boolean;
  title?: string;
  legend?: string;
}) {
  const L = useT().systems.layout3d;
  const place = usePlaceBody();
  const [selected, setSelected] = useState<string | null>(null);
  const [mode, setMode] = useState<TransformMode>("translate");
  const [snap, setSnap] = useState<string>(DEFAULT_SNAP.value);
  const step = (SNAPS.find((s) => s.value === snap) ?? DEFAULT_SNAP).step;
  const [showAxes, setShowAxes] = useState(true);
  const { boxes, unplaced } = boxesOf(design, colorOf);
  const part = design.parts.find((p) => p.id === selected) ?? null;
  return (
    <Panel pad={false} className="overflow-hidden">
      <div className="flex flex-wrap items-center gap-2 border-line border-b px-4 py-2">
        <h2 className="font-semibold text-md">{title ?? L.title}</h2>
        <span className="text-ink-3 text-xs">
          {L.parts(boxes.length)}
          {unplaced.length > 0 ? L.unplacedCount(unplaced.length) : ""}
          {legend ? ` · ${legend}` : ""}
          {com ? L.com : ""}
        </span>
        <Switch on={showAxes} onChange={setShowAxes} title={L.axesLegend}>
          {L.axes}
        </Switch>
        {showAxes && <span className="text-ink-3 text-xs">{L.axesLegend}</span>}
        {editable && (
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
        )}
      </div>
      <div className="grid h-[460px] grid-cols-[minmax(0,1fr)_340px]">
        <div className="relative min-w-0">
          {boxes.length === 0 ? (
            <Empty className="m-auto">{L.empty}</Empty>
          ) : (
            <Scene3D
              boxes={boxes}
              selected={selected}
              onSelect={setSelected}
              mode={mode}
              snap={step}
              com={com}
              axes={showAxes ? axesOf(design) : null}
              editable={editable}
              onPlace={(id, p, q) =>
                // 機体座標のまま送る。取付点に対する相対への換算はサーバー（POST /parts/{id}/place_body）
                place.mutate({
                  part: id,
                  position: { x: p[0], y: p[1], z: p[2], unit: "m" },
                  orientation: { w: q[0], x: q[1], y: q[2], z: q[3] },
                  summary: L.moved(id),
                })
              }
            />
          )}
        </div>
        <aside className="overflow-y-auto border-line border-l p-3">
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
