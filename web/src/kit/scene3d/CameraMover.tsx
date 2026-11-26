import { useBounds } from "@react-three/drei";
import { useFrame, useThree } from "@react-three/fiber";
import { useCallback, useEffect, useRef } from "react";
import { type Object3D, type PerspectiveCamera, Vector3 } from "three";
import type { FocusRequest, V3 } from "./types";
import {
  type CameraPose,
  type CubeView,
  easeMove,
  lerpPose,
  lookQuaternion,
  MOVE_SECONDS,
  poseForView,
  positionOf,
  rollPose,
  rollStep,
} from "./view-cube";

/** ビューキューブの区画を押した依頼（n が増えるたびに 1 回動かす） */
export interface ViewRequest {
  n: number;
  view: CubeView;
}

/**
 * 今見ている向きを軸に画面を回す依頼（n が増えるたびに 1 回。`turn` = 1 は画面の中の絵が時計回り、−1 は反時計回り）。
 * 1 回の角は見ている向きで決まる（`rollStep`: 角から見ていれば 120°、それ以外は 90°）
 */
export interface RollRequest {
  n: number;
  turn: 1 | -1;
}

/** 動かす注視点（`target`）だけを持つ視点の操作（`FreeOrbitControls`） */
interface Target {
  target: Vector3;
  addEventListener: (type: "start", listener: () => void) => void;
  removeEventListener: (type: "start", listener: () => void) => void;
  update: () => void;
}

/**
 * カメラを動かす唯一の経路。依頼は 4 つで、どれも今の姿勢から約 1 秒かけて滑らかに動かす
 * （注視点と距離は補間し、向きは注視点のまわりの弧を通る。`Bounds` の全体を収める動きと同じ緩急）:
 * - ホームポジション（`home` が増える）: 全体が収まる距離で、初めの向き（`position` から原点を見る向き、上は +Z）へ戻す
 * - ツリーで押した部品（`focus`）: 向きは今のまま、部品の外形に注視点と距離を合わせる（描いていない部品なら何もしない）
 * - ビューキューブの区画（`view`）: 注視点と距離はそのまま、表の向き・上の向きへ（押したあとは傾きのない画面になる。
 *   角は Y 字に見える 3 つの向きのうち今の向きに近いものへ）
 * - 視線まわりの回転（`roll`）: 注視点・距離・視線はそのまま、画面を 1 刻み回す（動いている途中で押したら、到達の姿勢からさらに回す）
 * ドラッグを始めたら（controls の `start`）動きを止める。frameloop="demand" なので、動いている間は描き直しを依頼し続ける。
 * `Bounds`（全体を収める範囲）の中に置く。
 */
export function CameraMover({
  home,
  position,
  focus,
  view,
  roll,
}: {
  home: number;
  /** 初めのカメラの位置（原点から見た向きだけを使う） */
  position: V3;
  focus: FocusRequest | null;
  view: ViewRequest | null;
  roll: RollRequest | null;
}) {
  const bounds = useBounds();
  const camera = useThree((s) => s.camera) as PerspectiveCamera;
  const scene = useThree((s) => s.scene);
  const invalidate = useThree((s) => s.invalidate);
  const controls = useThree((s) => s.controls) as Target | null;
  const move = useRef<{ from: CameraPose; to: CameraPose; start: number } | null>(null);
  // 同じ依頼を 2 回動かさない（controls や bounds が作り直されても、視点を戻さない）
  const done = useRef<{
    home: number;
    focus: FocusRequest | null;
    view: ViewRequest | null;
    roll: RollRequest | null;
  }>({ home, focus: null, view: null, roll: null });

  // 今の姿勢から、build が返す到達の姿勢へ動かし始める
  const begin = useCallback(
    (c: Target, build: (now: CameraPose) => CameraPose) => {
      const from: CameraPose = {
        target: c.target.clone(),
        quaternion: camera.quaternion.clone(),
        distance: camera.position.distanceTo(c.target),
      };
      move.current = { from, to: build(from), start: performance.now() };
      // 動かし始めを描き直しの依頼で起こす
      invalidate();
    },
    [camera, invalidate],
  );

  useEffect(() => {
    if (!controls) return;
    const stop = () => {
      move.current = null;
    };
    controls.addEventListener("start", stop);
    return () => controls.removeEventListener("start", stop);
  }, [controls]);

  const [x, y, z] = position;
  useEffect(() => {
    if (!bounds || !controls || done.current.home === home) return;
    done.current.home = home;
    const { center, distance } = bounds.refresh().clip().getSize();
    const direction = new Vector3(x, y, z).normalize();
    begin(controls, () => ({
      target: center,
      quaternion: lookQuaternion([direction.x, direction.y, direction.z], [0, 0, 1]),
      distance,
    }));
  }, [home, bounds, controls, begin, x, y, z]);

  useEffect(() => {
    if (!focus || !bounds || !controls || done.current.focus === focus) return;
    done.current.focus = focus;
    const object: Object3D | undefined = scene.getObjectByName(focus.id);
    if (!object) return;
    const { center, distance } = bounds.refresh(object).getSize();
    begin(controls, (now) => ({ ...now, target: center, distance }));
  }, [focus, bounds, controls, scene, begin]);

  useEffect(() => {
    if (!view || !controls || done.current.view === view) return;
    done.current.view = view;
    begin(controls, (now) => poseForView(view.view, now));
  }, [view, controls, begin]);

  useEffect(() => {
    if (!roll || !controls || done.current.roll === roll) return;
    done.current.roll = roll;
    // 続けて押したときに刻みが崩れないよう、動いている途中なら到達の姿勢から回す
    const going = move.current?.to;
    begin(controls, (now) => {
      const base = going ?? now;
      return rollPose(base, roll.turn * rollStep(base.quaternion));
    });
  }, [roll, controls, begin]);

  useFrame(() => {
    const m = move.current;
    if (!m || !controls) return;
    const t = (performance.now() - m.start) / 1000 / MOVE_SECONDS;
    const pose = lerpPose(m.from, m.to, easeMove(t));
    controls.target.copy(pose.target);
    camera.quaternion.copy(pose.quaternion);
    camera.position.copy(positionOf(pose));
    camera.updateMatrixWorld();
    if (t >= 1) move.current = null;
    controls.update();
    invalidate();
  });
  return null;
}
