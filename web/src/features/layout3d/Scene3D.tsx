import { Edges, Grid, TransformControls } from "@react-three/drei";
import { useThree } from "@react-three/fiber";
import { useEffect, useMemo, useRef, useState } from "react";
import type { Material, Mesh } from "three";
import { ArrowHelper, Color, DoubleSide, Quaternion, Vector3 } from "three";
import type { RollRequest } from "@/kit/scene3d/CameraMover";
import { SceneShell } from "@/kit/scene3d/SceneShell";
import type { ZoomRequest } from "@/kit/scene3d/ViewInput3D";
import { useHex } from "@/lib/theme";
import { paletteFor, type Scheme, toneHexFor, uiHexFor } from "@/lib/tone";
import {
  type Axis3D,
  axisColor,
  type Box3D,
  boxMaterial,
  FOV_OPACITY,
  type Fov3D,
  pickBox,
  type Q4,
  type SunArrow,
  type sceneDimensions,
  type V3,
} from "./scene";
import { DEFAULT_SCENE_HELPERS, type FocusRequest, type SceneHelpers } from "./visibility";

/** 押してから離すまでにこれより動いたらドラッグ（視点の操作）とみなし、クリックにしない [px] */
const CLICK_SLOP_PX = 4;

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
  helpers = DEFAULT_SCENE_HELPERS,
  focus = null,
  home = 0,
  zoom = null,
  roll = null,
  fovs = [],
  sun = null,
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
  /** 座標の表示（原点の軸・地面の格子・画面隅の軸）。省略するとすべて出す */
  helpers?: SceneHelpers;
  /** 視点を寄せる部品（ツリーで押したとき。描いていない部品なら何もしない） */
  focus?: FocusRequest | null;
  /** 視野の円錐（`helpers.fov` のときだけ描く） */
  fovs?: Fov3D[];
  /** 太陽の矢印（機体座標系の方向で、原点から描く。無ければ描かない。出し入れは呼び出し側が `helpers.sun` で決める） */
  sun?: SunArrow | null;
  /** ホームポジションに戻す依頼（押した回数。0 は依頼なし）。増えるたびに初めの向きへ戻し、全体が収まるまで引く */
  home?: number;
  /** ボタンからの拡大・縮小の依頼（`ViewInput3D`）。無ければ依頼なし */
  zoom?: ZoomRequest | null;
  /** ボタンからの、視線まわりに画面を回す依頼。無ければ依頼なし */
  roll?: RollRequest | null;
}) {
  const { scheme, tone, ui } = useHex();
  const [target, setTarget] = useState<Mesh | null>(null);
  // 押したときの画面の位置（離したときに動いていればドラッグ）
  const pressed = useRef<{ x: number; y: number } | null>(null);
  const dragged = (e: MouseEvent) =>
    pressed.current !== null &&
    Math.hypot(e.clientX - pressed.current.x, e.clientY - pressed.current.y) > CLICK_SLOP_PX;
  // TransformControls を掴んだときの位置・姿勢（離したときに動いたかを比べる）
  const grabbed = useRef<{ p: Vector3; q: Quaternion } | null>(null);
  const ids = new Set(boxes.map((b) => b.id));
  // 選んだ部品があれば、ほかの部品を淡く描く
  const dimmed = (id: string) => selected !== null && ids.has(selected) && id !== selected;
  return (
    // 枠（Canvas・照明・視点の操作・ビューキューブ）は `kit/scene3d/SceneShell`。ここは箱と、箱に付くものを渡す
    <SceneShell
      camera={{
        position: [dimensions.cameraX, dimensions.cameraY, dimensions.cameraZ],
        near: dimensions.near,
        far: dimensions.far,
      }}
      home={home}
      zoom={zoom}
      roll={roll}
      focus={focus}
      gizmo={helpers.gizmo}
      onPointerDown={(e) => {
        pressed.current = { x: e.clientX, y: e.clientY };
      }}
      onPointerMissed={(e) => {
        if (!dragged(e)) onSelect(null);
      }}
      behind={
        <>
          {helpers.grid && (
            <Grid
              rotation={[Math.PI / 2, 0, 0]}
              args={[dimensions.grid, dimensions.grid]}
              cellSize={dimensions.gridCell}
              sectionSize={dimensions.gridSection}
              cellColor={ui.tick}
              sectionColor={tone.gray}
              fadeDistance={dimensions.gridFade}
              infiniteGrid
            />
          )}
          {helpers.origin && <axesHelper args={[dimensions.axes]} />}
        </>
      }
      outside={
        <>
          {/* 視野は全体を収める範囲（Bounds）に入れない（広い円錐で機体が小さく映らないように）。部品の位置・姿勢に置く */}
          {helpers.fov &&
            fovs.map((f) => {
              const b = boxes.find((x) => x.id === f.part);
              return b ? (
                <group
                  key={f.part}
                  position={b.position}
                  quaternion={[
                    b.orientation[1],
                    b.orientation[2],
                    b.orientation[3],
                    b.orientation[0],
                  ]}
                >
                  <FovCone fov={f} length={dimensions.fovLength} color={b.color} />
                </group>
              ) : null;
            })}
          {sun && <SunArrowMark sun={sun} dimensions={dimensions} scheme={scheme} />}
          {com && <CenterOfMass position={com} radius={dimensions.comRadius} scheme={scheme} />}
          {edit && target && selected && (
            <TransformControls
              object={target}
              mode={edit.mode}
              translationSnap={edit.snap}
              rotationSnap={Math.PI / 12}
              size={0.7}
              onMouseDown={() => {
                grabbed.current = { p: target.position.clone(), q: target.quaternion.clone() };
              }}
              onMouseUp={() => {
                const p = target.position;
                const q = target.quaternion;
                const from = grabbed.current;
                grabbed.current = null;
                // 掴んで離しただけ（動いていない）なら書かない。押すだけで配置の変更が保存されないように
                if (from?.p.equals(p) && from.q.equals(q)) return;
                edit.onPlace(selected, [p.x, p.y, p.z], [q.w, q.x, q.y, q.z]);
              }}
            />
          )}
        </>
      }
    >
      {boxes.map((b) => (
        <mesh
          key={b.id}
          name={b.id}
          ref={b.id === selected ? setTarget : undefined}
          position={b.position}
          quaternion={[b.orientation[1], b.orientation[2], b.orientation[3], b.orientation[0]]}
          onClick={(e) => {
            e.stopPropagation();
            // 視点を回す・動かすドラッグの終わりはクリックとして扱わない（選択が奥へ移ったり外れたりしないように）
            if (dragged(e.nativeEvent)) return;
            // 光線上の当たり（近い順）のうち、箱の面（name = 部品の ID の mesh）に当たったものだけを数える。
            // 縁の線・軸の矢印の線は three の既定では 1 m 以内を通れば当たりになり、離れた部品を拾うため
            const hits = e.intersections
              .map((hit) => hit.object.name)
              .filter((name) => ids.has(name));
            // 箱の面に当たらなければ（縁の線などに当たっただけなら）、何も無い所を押したのと同じく選択を外す
            onSelect(pickBox(hits, selected));
          }}
        >
          {b.sized ? <boxGeometry args={b.size} /> : <octahedronGeometry args={[b.size[0] / 2]} />}
          <meshStandardMaterial
            {...boxMaterial(
              b,
              b.id === selected,
              structureOpacity,
              dimmed(b.id),
              helpers.wireframe,
            )}
          />
          <Edges
            color={b.id === selected ? ui.ink : dimmed(b.id) ? ui.tick : ui.ink3}
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
                    new Color(axisColor(scheme, a.order)),
                    dimensions.arrowHead,
                    dimensions.arrowWidth,
                  )
                }
              />
            ))}
        </mesh>
      ))}
    </SceneShell>
  );
}

/** 日陰のときの太陽の矢印の不透明度（消さずに薄く描き、向きは読めるようにする） */
const SUN_ECLIPSE_OPACITY = 0.25;

/**
 * 太陽の矢印: 機体の原点から太陽の向きへ。機体座標系の向きをそのまま three.js に渡す（箱・方向の属性の矢印と同じ座標。
 * 場面は機体座標系なので変換は要らない）。ArrowHelper は 1 つ作って、時刻が進むたびに向きと色だけ更新する（再生中に作り直さない）。
 * 全体を収める範囲（Bounds）には入れない。クリックの当たりには数えない。
 */
function SunArrowMark({
  sun,
  dimensions,
  scheme,
}: {
  sun: SunArrow;
  dimensions: ReturnType<typeof sceneDimensions>;
  scheme: Scheme;
}) {
  const tone = toneHexFor(scheme);
  const invalidate = useThree((state) => state.invalidate);
  const { sunArrow: length, arrowHead: head, arrowWidth: width } = dimensions;
  const arrow = useMemo(() => {
    const a = new ArrowHelper(new Vector3(0, 0, 1), new Vector3(), length, tone.gold, head, width);
    a.line.raycast = () => null;
    a.cone.raycast = () => null;
    return a;
  }, [length, head, width, tone.gold]);
  useEffect(() => () => arrow.dispose(), [arrow]);
  const [x, y, z] = sun.dir;
  const { eclipse } = sun;
  useEffect(() => {
    const dir = new Vector3(x, y, z);
    if (dir.lengthSq() === 0) return;
    arrow.setDirection(dir.normalize());
    arrow.setColor(eclipse ? tone.gray : tone.gold);
    for (const m of [arrow.line.material as Material, arrow.cone.material as Material]) {
      m.transparent = eclipse;
      m.opacity = eclipse ? SUN_ECLIPSE_OPACITY : 1;
    }
    // frameloop="demand" では、向きの更新を描き直しの依頼で反映する
    invalidate();
  }, [arrow, x, y, z, eclipse, tone.gray, tone.gold, invalidate]);
  return <primitive object={arrow} />;
}

/** 重心の記号の 8 分割（経度 4 × 緯度 2）。隣り合う区画が白と黒になるよう互い違いに塗る */
const COM_OCTANTS = [0, 1, 2, 3].flatMap((i) =>
  [0, 1].map((j) => ({ i, j, dark: (i + j) % 2 === 0 })),
);

/**
 * 重心の記号（CAD の重心マーク）: 球を 8 つに分けて白と黒を互い違いに塗る。部品に隠れても見えるよう奥行きの判定をしない。
 */
function CenterOfMass({
  position,
  radius,
  scheme,
}: {
  position: V3;
  radius: number;
  scheme: Scheme;
}) {
  const ui = uiHexFor(scheme);
  return (
    // 球の極（three の球は Y 軸）を機体の +Z に向け、白黒の境目の 1 本を水平にする
    <group position={position} rotation={[Math.PI / 2, 0, 0]}>
      {COM_OCTANTS.map(({ i, j, dark }) => (
        <mesh key={`${i}-${j}`} renderOrder={10}>
          <sphereGeometry
            args={[radius, 12, 8, (i * Math.PI) / 2, Math.PI / 2, (j * Math.PI) / 2, Math.PI / 2]}
          />
          <meshBasicMaterial color={dark ? ui.ink : paletteFor(scheme).inverse} depthTest={false} />
        </mesh>
      ))}
    </group>
  );
}

const UP = new Vector3(0, 1, 0);

/**
 * 視野の円錐（部品の位置・姿勢の group の子。部品座標。色は部品の箱と同じ）。頂点を部品の原点に、底を視線の向きに置く。three の円錐は +Y が頂点なので、
 * +Y を視線の逆向きに回し、中心を視線の向きに高さの半分ずらす。クリックの当たりには数えない。
 */
function FovCone({ fov, length, color }: { fov: Fov3D; length: number; color: string }) {
  const dir = new Vector3(...fov.dir);
  const center = dir.clone().multiplyScalar(length / 2);
  const rotation = new Quaternion().setFromUnitVectors(UP, dir.clone().negate());
  return (
    <mesh position={center} quaternion={rotation} raycast={() => null}>
      <coneGeometry args={[length * Math.tan(fov.halfAngle), length, 32, 1, true]} />
      <meshBasicMaterial
        color={color}
        transparent
        opacity={FOV_OPACITY}
        depthWrite={false}
        side={DoubleSide}
      />
    </mesh>
  );
}
