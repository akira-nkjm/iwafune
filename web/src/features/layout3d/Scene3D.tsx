import {
  Bounds,
  Edges,
  GizmoHelper,
  GizmoViewport,
  Grid,
  OrbitControls,
  TransformControls,
} from "@react-three/drei";
import { Canvas } from "@react-three/fiber";
import { useState } from "react";
import type { Mesh } from "three";
import { ArrowHelper, Color, Vector3 } from "three";
import { palette, toneHex, uiHex } from "@/lib/tone";
import type { Axis3D, Box3D, Q4, V3 } from "./scene";

const AXIS_COLOR: Record<Axis3D["attr"], string> = {
  spin_axis: toneHex.blue,
  axis: toneHex.teal,
  normal: toneHex.gold,
  boresight: toneHex.red,
  direction: toneHex.navy,
};

export type TransformMode = "translate" | "rotate";

/** 機体座標系（+Z を上）で部品の箱を描く。選んだ箱には TransformControls を付け、離したときに onPlace を 1 回呼ぶ */
export function Scene3D({
  boxes,
  selected,
  onSelect,
  onPlace,
  mode,
  snap,
  com,
  editable,
  axes,
}: {
  boxes: Box3D[];
  selected: string | null;
  onSelect: (id: string | null) => void;
  onPlace?: (id: string, position: V3, orientation: Q4) => void;
  mode: TransformMode;
  /** 並進のスナップ [m] */
  snap: number;
  /** 重心（機体座標、m） */
  com?: V3 | null;
  editable: boolean;
  /** 方向ベクトルの矢印（null なら描かない） */
  axes?: Axis3D[] | null;
}) {
  const [target, setTarget] = useState<Mesh | null>(null);
  return (
    // demand: 何かが変わったとき（カメラの操作・箱の移動・props の変更）だけ描き直す。
    // 既定の always は止まっている間も毎フレーム描き、ページ全体の操作を重くする
    <Canvas
      frameloop="demand"
      camera={{ position: [0.6, -0.7, 0.5], up: [0, 0, 1], fov: 40, near: 0.001, far: 100 }}
      onPointerMissed={() => onSelect(null)}
      style={{ background: "var(--color-panel-2)" }}
    >
      <ambientLight intensity={0.7} />
      <directionalLight position={[2, -3, 4]} intensity={1.2} />
      <Grid
        rotation={[Math.PI / 2, 0, 0]}
        args={[2, 2]}
        cellSize={0.01}
        sectionSize={0.1}
        cellColor={uiHex.tick}
        sectionColor={toneHex.gray}
        fadeDistance={3}
        infiniteGrid
      />
      <axesHelper args={[0.15]} />
      <Bounds fit clip margin={1.3}>
        {boxes.map((b) => (
          <mesh
            key={b.id}
            ref={b.id === selected ? setTarget : undefined}
            position={b.position}
            quaternion={[b.orientation[1], b.orientation[2], b.orientation[3], b.orientation[0]]}
            onClick={(e) => {
              e.stopPropagation();
              onSelect(b.id);
            }}
          >
            <boxGeometry args={b.size} />
            <meshStandardMaterial color={b.color} transparent opacity={b.sized ? 0.85 : 0.45} />
            <Edges
              color={b.id === selected ? uiHex.ink : uiHex.ink3}
              lineWidth={b.id === selected ? 2 : 1}
            />
            {axes
              ?.filter((a) => a.part === b.id)
              .map((a) => (
                <primitive
                  key={a.id}
                  object={
                    new ArrowHelper(
                      new Vector3(...a.dir),
                      new Vector3(),
                      0.08,
                      new Color(AXIS_COLOR[a.attr]),
                      0.02,
                      0.012,
                    )
                  }
                />
              ))}
          </mesh>
        ))}
      </Bounds>
      {com && (
        <mesh position={com} renderOrder={10}>
          <sphereGeometry args={[0.008, 16, 16]} />
          <meshBasicMaterial color={toneHex.red} depthTest={false} />
        </mesh>
      )}
      {editable && target && selected && (
        <TransformControls
          object={target}
          mode={mode}
          translationSnap={snap}
          rotationSnap={Math.PI / 12}
          size={0.7}
          onMouseUp={() => {
            const p = target.position;
            const q = target.quaternion;
            onPlace?.(selected, [p.x, p.y, p.z], [q.w, q.x, q.y, q.z]);
          }}
        />
      )}
      {/* makeDefault: TransformControls を掴んでいる間は drei がカメラ操作を止める */}
      <OrbitControls makeDefault />
      <GizmoHelper alignment="bottom-right" margin={[64, 64]}>
        <GizmoViewport labelColor={palette.inverse} axisHeadScale={0.9} />
      </GizmoHelper>
    </Canvas>
  );
}
