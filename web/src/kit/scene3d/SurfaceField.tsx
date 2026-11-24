import { useFrame, useThree } from "@react-three/fiber";
import { useCallback, useEffect, useMemo, useRef } from "react";
import { BufferAttribute, BufferGeometry, DoubleSide, DynamicDrawUsage } from "three";
import type { SurfaceMesh } from "@/api/types";
import { useHex } from "@/lib/theme";
import { fieldRgb } from "./field-color";
import { SceneShell, useViewRequests } from "./SceneShell";
import {
  deformedPositions,
  edgeIndices,
  type FieldAt,
  flatPositions,
  gatherPositions,
  meshExtent,
  nodeColors,
  quadColors,
  quadEdges,
  quadOfFace,
  quadTriangleIndices,
  quadVertexNodes,
  triangleIndices,
} from "./surface-mesh";
import type { V3 } from "./types";
import { ViewControls3D } from "./ViewControls3D";

/** アニメーションの 1 周期 [s]（実時間の振動ではなく、見て分かる速さ） */
const PERIOD_S = 2.4;
/** 1 コマで進める時間の上限 [s] */
const MAX_FRAME_S = 0.1;

/** 場の置き方と動き（`SurfaceField` と `FieldMesh` が共有する props） */
interface FieldProps {
  mesh: SurfaceMesh;
  /** 場の値（0 〜 1。色に写す）。`at` が `"node"` なら節点ごと（`mesh.nodes` の並び）、`"cell"` なら四角形ごと（パネルの順の通し番号） */
  values: readonly number[];
  /** 場の値の置き場所。既定は節点ごと（滑らかに塗る）。`"cell"` は四角形を 1 色で塗る（升の温度など） */
  at?: FieldAt;
  /** 節点ごとの変位（`mesh.nodes` の並び。単位は規格化したもの）。無ければメッシュは動かない */
  displacement?: readonly V3[];
  /** 強調する四角形の通し番号（ホバー中の升など） */
  highlight?: number | null;
  /** カーソルが乗った四角形の通し番号（外れたら null）。渡したときだけ、面が当たりを受ける */
  onHover?: (quad: number | null) => void;
}

/**
 * メッシュ上の場の 3D 表示（共通の部品。`kit/scene3d/SceneShell` の枠）: 場の値（節点ごとか、四角形ごと）の色でメッシュを塗り、
 * 辺を薄い線で描く。変位があれば、元のメッシュを薄い線、変形したメッシュを場の値の色で描く（モードの形）。
 * 変形した位置 = 節点の位置 + 倍率 × 変位（表示のための足し算）。変位・場の値・規格化は API が決めたもので、ここでは計算しない。
 * 色の配色は `field-color.ts`（`lib/tone.ts` の `heatHex`。熱画像と同じ）。
 * `animate` なら倍率を sin で往復させる（止めると最大の変形で止まる）。frameloop="demand" なので、動かす間は自分で次のコマを依頼する。
 */
export function SurfaceField({
  mesh,
  values,
  at = "node",
  displacement,
  amplitude = 0,
  animate = false,
  highlight = null,
  onHover,
  className,
}: FieldProps & {
  /** 最大の変位を表示する大きさ（メッシュの大きさに対する比。表示だけの量） */
  amplitude?: number;
  animate?: boolean;
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
          values={values}
          at={at}
          displacement={displacement}
          highlight={highlight}
          onHover={onHover}
          scale={amplitude * extent}
          animate={animate}
        />
      </SceneShell>
      <ViewControls3D onZoom={zoomBy} onHome={goHome} />
    </div>
  );
}

/** 塗る面の頂点・色・三角形。節点ごとは節点を共有して滑らかに、四角形ごとは頂点を分けて 1 枚ずつ 1 色にする */
function useSurface(
  mesh: SurfaceMesh,
  values: readonly number[],
  at: FieldAt,
  position: BufferAttribute,
) {
  const cell = at === "cell";
  const nodeOf = useMemo(() => (cell ? quadVertexNodes(mesh) : null), [cell, mesh]);
  // 四角形ごとの頂点の位置（節点の位置を引き写したもの。節点ごとのときは節点の位置をそのまま使う）。
  // 最初から正しい位置で作る（全体を収める範囲や当たり判定の境界が、最初の 0 の位置で決まらないように）
  const vertices = useMemo(
    () =>
      nodeOf
        ? new BufferAttribute(
            gatherPositions(
              position.array as Float32Array,
              nodeOf,
              new Float32Array(nodeOf.length * 3),
            ),
            3,
          ).setUsage(DynamicDrawUsage)
        : position,
    [nodeOf, position],
  );
  const surface = useMemo(() => {
    const g = new BufferGeometry();
    g.setAttribute("position", vertices);
    g.setAttribute(
      "color",
      new BufferAttribute(cell ? quadColors(values, fieldRgb) : nodeColors(values, fieldRgb), 3),
    );
    g.setIndex(
      new BufferAttribute(cell ? quadTriangleIndices(values.length) : triangleIndices(mesh), 1),
    );
    return g;
  }, [vertices, values, cell, mesh]);
  useEffect(() => () => surface.dispose(), [surface]);
  return { surface, vertices, nodeOf };
}

function FieldMesh({
  mesh,
  values,
  at = "node",
  displacement,
  highlight = null,
  onHover,
  scale,
  animate,
}: FieldProps & { scale: number; animate: boolean }) {
  const { ui } = useHex();
  const invalidate = useThree((s) => s.invalidate);
  const nodes = mesh.nodes as V3[];
  const phase = useRef(0);
  // 位置の配列は 1 つ。変形した面と変形した辺が同じ属性を共有し、更新は 1 回で済む
  const position = useMemo(
    () => new BufferAttribute(flatPositions(nodes), 3).setUsage(DynamicDrawUsage),
    [nodes],
  );
  const { surface, vertices, nodeOf } = useSurface(mesh, values, at, position);
  const edges = useMemo(() => edgeIndices(mesh), [mesh]);
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
  const outline = useMemo(() => {
    const g = new BufferGeometry();
    g.setAttribute("position", position);
    g.setIndex(
      new BufferAttribute(highlight === null ? new Uint32Array(0) : quadEdges(mesh, highlight), 1),
    );
    return g;
  }, [position, mesh, highlight]);
  useEffect(
    () => () => {
      deformedEdges.dispose();
      restEdges.dispose();
      outline.dispose();
    },
    [deformedEdges, restEdges, outline],
  );
  const apply = useCallback(
    (factor: number) => {
      deformedPositions(nodes, displacement ?? [], scale * factor, position.array as Float32Array);
      position.needsUpdate = true;
      if (nodeOf) {
        gatherPositions(position.array as Float32Array, nodeOf, vertices.array as Float32Array);
        vertices.needsUpdate = true;
        // 当たり判定（ホバー）が使う境界は、位置が動いたら取り直す（三角形の頂点の位置と食い違わないように）
        surface.computeBoundingBox();
        surface.computeBoundingSphere();
      }
      invalidate();
    },
    [nodes, displacement, scale, position, vertices, nodeOf, surface, invalidate],
  );
  // 止めているときは最大の変形（倍率 1）。変位・倍率・止める／動かすが変わったら 1 回描き直す（動かすときは次のコマも依頼される）
  useEffect(() => {
    apply(animate && displacement ? Math.sin(phase.current) : 1);
  }, [apply, animate, displacement]);
  useFrame((_, dt) => {
    if (!animate || !displacement) return;
    // 描き直しは必要なときだけなので、止まっていたあとの最初のコマの dt は大きい。位相が跳ばないように上限を置く
    phase.current += (Math.min(dt, MAX_FRAME_S) * 2 * Math.PI) / PERIOD_S;
    apply(Math.sin(phase.current));
  });
  return (
    <>
      {displacement && (
        <lineSegments geometry={restEdges} frustumCulled={false} raycast={() => null}>
          <lineBasicMaterial color={ui.ink3} transparent opacity={0.3} />
        </lineSegments>
      )}
      <mesh
        geometry={surface}
        frustumCulled={false}
        {...(onHover ? {} : { raycast: () => null })}
        onPointerMove={
          onHover
            ? (e) => {
                e.stopPropagation();
                if (e.faceIndex != null) onHover(quadOfFace(e.faceIndex));
              }
            : undefined
        }
        onPointerOut={onHover ? () => onHover(null) : undefined}
      >
        {/* 面の奥行きを少し後ろにずらす: 同じ面の上の辺の線が、面と重なってちらつかないように */}
        <meshBasicMaterial
          vertexColors
          side={DoubleSide}
          polygonOffset
          polygonOffsetFactor={1}
          polygonOffsetUnits={1}
        />
      </mesh>
      <lineSegments geometry={deformedEdges} frustumCulled={false} raycast={() => null}>
        <lineBasicMaterial color={ui.ink} transparent opacity={displacement ? 0.22 : 0.45} />
      </lineSegments>
      {highlight !== null && (
        <lineSegments geometry={outline} frustumCulled={false} raycast={() => null}>
          <lineBasicMaterial color={ui.ink} depthTest={false} />
        </lineSegments>
      )}
    </>
  );
}
