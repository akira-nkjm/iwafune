import { useFrame, useThree } from "@react-three/fiber";
import { useCallback, useEffect, useMemo, useRef } from "react";
import { BufferAttribute, BufferGeometry, Color, DoubleSide, DynamicDrawUsage } from "three";
import type { SurfaceMesh } from "@/api/types";
import { useHex } from "@/lib/theme";
import { heatHex } from "@/lib/tone";
import { SceneShell, useViewRequests } from "./SceneShell";
import {
  deformedPositions,
  edgeIndices,
  flatPositions,
  meshExtent,
  nodeColors,
  triangleIndices,
} from "./surface-mesh";
import type { V3 } from "./types";
import { ViewControls3D } from "./ViewControls3D";

/** 配色の段数（`heatHex` を一度だけ引いて表にする。凡例と同じ配色） */
const LUT_STEPS = 64;
const LUT: readonly (readonly [number, number, number])[] = Array.from(
  { length: LUT_STEPS + 1 },
  (_, i) => new Color(heatHex(i / LUT_STEPS)).toArray() as [number, number, number],
);

/** 場の値（0 〜 1。範囲外は端）→ 色（RGB 0 〜 1）。配色は `lib/tone.ts` の `heatHex`（熱画像と同じ連続の配色） */
function rgbOf(ratio: number): readonly [number, number, number] {
  const i = Math.round(Math.min(1, Math.max(0, ratio)) * LUT_STEPS);
  return LUT[i] ?? [0, 0, 0];
}

/** アニメーションの 1 周期 [s]（実時間の振動ではなく、見て分かる速さ） */
const PERIOD_S = 2.4;
/** 1 コマで進める時間の上限 [s] */
const MAX_FRAME_S = 0.1;

/**
 * メッシュ上の場の 3D 表示（共通の部品。`kit/scene3d/SceneShell` の枠）: 元のメッシュを薄い線で、変形したメッシュを場の値の色で描く。
 * 変形した位置 = 節点の位置 + 倍率 × 変位（表示のための足し算）。変位・場の値・規格化は API が決めたもので、ここでは計算しない。
 * `animate` なら倍率を sin で往復させる（止めると最大の変形で止まる）。frameloop="demand" なので、動かす間は自分で次のコマを依頼する。
 */
export function SurfaceField({
  mesh,
  displacement,
  values,
  amplitude,
  animate,
  className,
}: {
  mesh: SurfaceMesh;
  /** 節点ごとの変位（`mesh.nodes` の並び。単位は規格化したもの） */
  displacement: readonly V3[];
  /** 節点ごとの場の値（0 〜 1。色に写す） */
  values: readonly number[];
  /** 最大の変位を表示する大きさ（メッシュの大きさに対する比。表示だけの量） */
  amplitude: number;
  animate: boolean;
  className?: string;
}) {
  const nodes = mesh.nodes as V3[];
  const extent = useMemo(() => meshExtent(nodes), [nodes]);
  const { home, zoom, zoomBy, goHome } = useViewRequests();
  return (
    <div className={className ?? "relative h-full min-h-[320px]"}>
      <SceneShell
        camera={{
          position: [extent * 0.9, -extent * 1.3, extent * 0.8],
          near: extent * 0.01,
          far: extent * 100,
        }}
        home={home}
        zoom={zoom}
      >
        <FieldMesh
          mesh={mesh}
          displacement={displacement}
          values={values}
          scale={amplitude * extent}
          animate={animate}
        />
      </SceneShell>
      <ViewControls3D onZoom={zoomBy} onHome={goHome} />
    </div>
  );
}

function FieldMesh({
  mesh,
  displacement,
  values,
  scale,
  animate,
}: {
  mesh: SurfaceMesh;
  displacement: readonly V3[];
  values: readonly number[];
  scale: number;
  animate: boolean;
}) {
  const { ui } = useHex();
  const invalidate = useThree((s) => s.invalidate);
  const nodes = mesh.nodes as V3[];
  const phase = useRef(0);
  // 位置の配列は 1 つ。変形した面と変形した辺が同じ属性を共有し、更新は 1 回で済む
  const position = useMemo(
    () => new BufferAttribute(flatPositions(nodes), 3).setUsage(DynamicDrawUsage),
    [nodes],
  );
  const triangles = useMemo(() => triangleIndices(mesh), [mesh]);
  const edges = useMemo(() => edgeIndices(mesh), [mesh]);
  const surface = useMemo(() => {
    const g = new BufferGeometry();
    g.setAttribute("position", position);
    g.setAttribute("color", new BufferAttribute(nodeColors(values, rgbOf), 3));
    g.setIndex(new BufferAttribute(triangles, 1));
    return g;
  }, [position, values, triangles]);
  const deformedEdges = useMemo(() => {
    const g = new BufferGeometry();
    g.setAttribute("position", position);
    g.setIndex(new BufferAttribute(edges, 1));
    return g;
  }, [position, edges]);
  const restEdges = useMemo(() => {
    const g = new BufferGeometry();
    g.setAttribute("position", new BufferAttribute(flatPositions(nodes), 3));
    g.setIndex(new BufferAttribute(edges, 1));
    return g;
  }, [nodes, edges]);
  useEffect(
    () => () => {
      surface.dispose();
      deformedEdges.dispose();
      restEdges.dispose();
    },
    [surface, deformedEdges, restEdges],
  );
  const apply = useCallback(
    (factor: number) => {
      deformedPositions(nodes, displacement, scale * factor, position.array as Float32Array);
      position.needsUpdate = true;
      invalidate();
    },
    [nodes, displacement, scale, position, invalidate],
  );
  // 止めているときは最大の変形（倍率 1）。変位・倍率・止める／動かすが変わったら 1 回描き直す（動かすときは次のコマも依頼される）
  useEffect(() => {
    apply(animate ? Math.sin(phase.current) : 1);
  }, [apply, animate]);
  useFrame((_, dt) => {
    if (!animate) return;
    // 描き直しは必要なときだけなので、止まっていたあとの最初のコマの dt は大きい。位相が跳ばないように上限を置く
    phase.current += (Math.min(dt, MAX_FRAME_S) * 2 * Math.PI) / PERIOD_S;
    apply(Math.sin(phase.current));
  });
  return (
    <>
      <lineSegments geometry={restEdges} frustumCulled={false} raycast={() => null}>
        <lineBasicMaterial color={ui.ink3} transparent opacity={0.3} />
      </lineSegments>
      <mesh geometry={surface} frustumCulled={false} raycast={() => null}>
        <meshBasicMaterial vertexColors side={DoubleSide} />
      </mesh>
      <lineSegments geometry={deformedEdges} frustumCulled={false} raycast={() => null}>
        <lineBasicMaterial color={ui.ink} transparent opacity={0.22} />
      </lineSegments>
    </>
  );
}

/** 凡例の帯（CSS の線形グラデーション。色は 3D と同じ `heatHex`） */
export function fieldGradient(): string {
  return `linear-gradient(to right, ${Array.from({ length: 8 }, (_, i) => heatHex(i / 7)).join(", ")})`;
}

/** 凡例（低い値 → 高い値の帯）。端の文字は呼び出し側が渡す（言葉は辞書） */
export function FieldLegend({ low, high, label }: { low: string; high: string; label: string }) {
  return (
    <div className="flex items-center gap-2 text-ink-2 text-xs" role="img" aria-label={label}>
      <span>{low}</span>
      <span className="h-3 w-48 max-w-full rounded-sm" style={{ background: fieldGradient() }} />
      <span>{high}</span>
    </div>
  );
}
