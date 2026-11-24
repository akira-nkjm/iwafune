import { Bounds, GizmoHelper } from "@react-three/drei";
import { Canvas } from "@react-three/fiber";
import { type ComponentProps, type ReactNode, useCallback, useReducer, useState } from "react";
import { useT } from "@/i18n";
import { useHex } from "@/lib/theme";
import { CameraMover, type ViewRequest } from "./CameraMover";
import { FreeOrbit } from "./FreeOrbit";
import type { FocusRequest, V3 } from "./types";
import { ViewCube } from "./ViewCube";
import { ViewInput3D, type ZoomRequest } from "./ViewInput3D";
import type { CubeView } from "./view-cube";

/** 視点の操作ボタン（拡大・縮小・ホームポジション）の依頼。`ViewControls3D` に渡し、`SceneShell` が読む */
export function useViewRequests() {
  const [home, setHome] = useState(0);
  // ボタンの拡大・縮小の依頼（押すたびに n が増える）
  const [zoom, zoomBy] = useReducer(
    (old: ZoomRequest | null, factor: number) => ({ n: (old?.n ?? 0) + 1, factor }),
    null,
  );
  const goHome = useCallback(() => setHome((n) => n + 1), []);
  return { home, zoom, zoomBy, goHome };
}

/** 場面の置き方（カメラの初めの位置と、描く範囲の手前・奥の限界。場面の大きさから呼び出し側が決める） */
export interface SceneCamera {
  position: V3;
  near: number;
  far: number;
}

/**
 * 3D の場面の共通の枠: Canvas（機体座標系、+Z が上）・照明・全体を収める範囲（`Bounds`）とカメラの動き・自由な視点の操作・
 * ビューキューブ。配置・熱画像・メッシュ上の場（モードの形など）が同じ枠を使う。描くものは `children`（全体を収める範囲に入る）、
 * 範囲に入れないもの（格子・軸・記号）は `behind`（箱より先）と `outside`（箱のあと）に渡す。
 * frameloop="demand": 何かが変わったとき（カメラの操作・props の変更）だけ描き直す。動かし続けたい中身は自分で `invalidate` する。
 */
export function SceneShell({
  camera,
  home = 0,
  zoom = null,
  focus = null,
  gizmo = true,
  onPointerDown,
  onPointerMissed,
  behind,
  outside,
  children,
}: {
  camera: SceneCamera;
  /** ホームポジションに戻す依頼（押した回数。0 は依頼なし） */
  home?: number;
  /** ボタンからの拡大・縮小の依頼 */
  zoom?: ZoomRequest | null;
  /** 視点を寄せる部品（描いていなければ何もしない） */
  focus?: FocusRequest | null;
  /** 画面隅のビューキューブを出すか */
  gizmo?: boolean;
  onPointerDown?: ComponentProps<typeof Canvas>["onPointerDown"];
  onPointerMissed?: ComponentProps<typeof Canvas>["onPointerMissed"];
  behind?: ReactNode;
  outside?: ReactNode;
  children: ReactNode;
}) {
  const { scheme } = useHex();
  const L = useT().systems.layout3d;
  // ビューキューブの区画を押した依頼（押すたびに n が増える。動かすのは CameraMover）
  const [view, setView] = useState<ViewRequest | null>(null);
  const pickView = useCallback(
    (v: CubeView) => setView((prev) => ({ n: (prev?.n ?? 0) + 1, view: v })),
    [],
  );
  const cubeLabel = useCallback((name: string) => L.viewCubeFace(name), [L]);
  return (
    <Canvas
      frameloop="demand"
      camera={{
        position: camera.position,
        up: [0, 0, 1],
        fov: 40,
        near: camera.near,
        far: camera.far,
      }}
      onPointerDown={onPointerDown}
      onPointerMissed={onPointerMissed}
      style={{ background: "var(--color-panel-2)" }}
    >
      <ambientLight intensity={0.7} />
      <directionalLight position={[2, -3, 4]} intensity={1.2} />
      {behind}
      <Bounds fit clip margin={1.3}>
        <CameraMover home={home} position={camera.position} focus={focus} view={view} />
        {children}
      </Bounds>
      {outside}
      {/* 視点の操作を controls に入れる: TransformControls を掴んでいる間は drei がその操作を止める */}
      {/* 左ドラッグ = どの軸まわりにも回す、中・右ドラッグと Shift + 左ドラッグ = 移動。ホイールは ViewInput3D が読む */}
      <FreeOrbit />
      <ViewInput3D request={zoom} />
      {gizmo && (
        <GizmoHelper alignment="bottom-right" margin={[64, 64]}>
          <ViewCube scheme={scheme} onPick={pickView} label={cubeLabel} />
        </GizmoHelper>
      )}
    </Canvas>
  );
}
