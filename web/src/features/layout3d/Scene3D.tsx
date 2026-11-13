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
import {
  type Axis3D,
  type Box3D,
  boxMaterial,
  type Q4,
  type sceneDimensions,
  type V3,
} from "./scene";

const AXIS_COLOR: Record<Axis3D["attr"], string> = {
  spin_axis: toneHex.blue,
  axis: toneHex.teal,
  normal: toneHex.gold,
  boresight: toneHex.red,
  direction: toneHex.navy,
};

export type TransformMode = "translate" | "rotate";

/** 箱を動かす編集（構造の配置）。操作の種類・並進のスナップと、離したときに 1 回呼ぶ `onPlace` */
export interface SceneEdit {
  mode: TransformMode;
  /** 並進のスナップ [m] */
  snap: number;
  /** 動かし終えた箱の機体座標の位置 [m] と姿勢 [w, x, y, z] */
  onPlace: (id: string, position: V3, orientation: Q4) => void;
}

/**
 * 機体座標系（+Z を上）で部品の箱を描く。`edit` があれば、選んだ箱に TransformControls を付け、離したときに
 * `edit.onPlace` を 1 回呼ぶ（無ければ選ぶだけ）。
 */
export function Scene3D({
  boxes,
  dimensions,
  selected,
  onSelect,
  com,
  axes,
  edit,
  structureOpacity,
}: {
  boxes: Box3D[];
  dimensions: ReturnType<typeof sceneDimensions>;
  selected: string | null;
  onSelect: (id: string | null) => void;
  /** 重心（機体座標、m） */
  com?: V3 | null;
  /** 方向ベクトルの矢印（null なら描かない） */
  axes?: Axis3D[] | null;
  /** 箱を動かせるとき（無ければ表示だけ） */
  edit?: SceneEdit;
  /** 構造の不透明度。省略すると従来の熱の表示を保つ。 */
  structureOpacity?: number;
}) {
  const [target, setTarget] = useState<Mesh | null>(null);
  return (
    // demand: 何かが変わったとき（カメラの操作・箱の移動・props の変更）だけ描き直す。
    // 既定の always は止まっている間も毎フレーム描き、ページ全体の操作を重くする
    <Canvas
      frameloop="demand"
      camera={{
        position: [dimensions.cameraX, dimensions.cameraY, dimensions.cameraZ],
        up: [0, 0, 1],
        fov: 40,
        near: dimensions.near,
        far: dimensions.far,
      }}
      onPointerMissed={() => onSelect(null)}
      style={{ background: "var(--color-panel-2)" }}
    >
      <ambientLight intensity={0.7} />
      <directionalLight position={[2, -3, 4]} intensity={1.2} />
      <Grid
        rotation={[Math.PI / 2, 0, 0]}
        args={[dimensions.grid, dimensions.grid]}
        cellSize={dimensions.gridCell}
        sectionSize={dimensions.gridSection}
        cellColor={uiHex.tick}
        sectionColor={toneHex.gray}
        fadeDistance={dimensions.gridFade}
        infiniteGrid
      />
      <axesHelper args={[dimensions.axes]} />
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
            <meshStandardMaterial {...boxMaterial(b, b.id === selected, structureOpacity)} />
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
                      dimensions.arrow,
                      new Color(AXIS_COLOR[a.attr]),
                      dimensions.arrowHead,
                      dimensions.arrowWidth,
                    )
                  }
                />
              ))}
          </mesh>
        ))}
      </Bounds>
      {com && (
        <mesh position={com} renderOrder={10}>
          <sphereGeometry args={[dimensions.comRadius, 16, 16]} />
          <meshBasicMaterial color={toneHex.red} depthTest={false} />
        </mesh>
      )}
      {edit && target && selected && (
        <TransformControls
          object={target}
          mode={edit.mode}
          translationSnap={edit.snap}
          rotationSnap={Math.PI / 12}
          size={0.7}
          onMouseUp={() => {
            const p = target.position;
            const q = target.quaternion;
            edit.onPlace(selected, [p.x, p.y, p.z], [q.w, q.x, q.y, q.z]);
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
