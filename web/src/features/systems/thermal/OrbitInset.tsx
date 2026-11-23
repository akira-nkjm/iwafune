import { useMemo } from "react";
import type { OrbitPlaneTrack } from "@/api/types";
import { useT } from "@/i18n";
import { Formatted } from "@/quantity/Formatted";
import { INSET_SIZE, insetLayout, satelliteAt } from "./insetGeometry";

/**
 * 軌道の図（熱画像の右上）: 中心天体・軌道・影の帯（−x 側）・太陽（+x 側）と、今の時刻の衛星。
 * 座標はサーバーが軌道面の表示枠に直したもの（`orbit_plane`）をそのまま倍率だけかけて描く。日陰かどうかは API の日照率で、
 * 影の帯は模式（円柱影）。時刻（`index`）が進んでも、軌道の線は作り直さず、衛星の点だけ動かす。
 */
export function OrbitInset({ plane, index }: { plane: OrbitPlaneTrack; index: number }) {
  const I = useT().systems.thermalImage.orbit;
  const layout = useMemo(() => insetLayout(plane), [plane]);
  if (!layout) return null;
  const sat = satelliteAt(layout, plane, index);
  const { size, cx, cy } = layout;
  return (
    <figure className="m-0 space-y-1 text-xs">
      <svg
        viewBox={`0 0 ${size} ${size}`}
        width={INSET_SIZE}
        height={INSET_SIZE}
        role="img"
        aria-label={I.label}
        className="block"
      >
        <rect {...layout.shadow} className="fill-ink-3" fillOpacity={0.18} />
        <path d={layout.path} fill="none" className="stroke-ink-4" strokeWidth={1} />
        <circle cx={cx} cy={cy} r={layout.bodyRadius} className="fill-sunken stroke-line-strong" />
        <circle cx={layout.sun.x} cy={layout.sun.y} r={5} className="fill-cat-gold" />
        <circle
          cx={sat.x}
          cy={sat.y}
          r={4}
          data-eclipse={sat.eclipse}
          className={sat.eclipse ? "fill-ink-4" : "fill-accent"}
          fillOpacity={sat.eclipse ? 0.6 : 1}
        />
      </svg>
      <figcaption className="flex items-center justify-between gap-2 text-ink-2">
        <span>{sat.eclipse ? I.eclipse : I.sunlit}</span>
        <span className="text-ink-3">
          {I.beta} <Formatted si={plane.sun_elevation} dimension="Angle" fallback="" />
        </span>
      </figcaption>
    </figure>
  );
}
