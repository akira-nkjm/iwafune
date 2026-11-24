import { useThree } from "@react-three/fiber";
import { useEffect, useRef } from "react";
import { PerspectiveCamera, Vector3 } from "three";
import { clampZoom, createWheelClassifier, wheelPixels, zoomFactor } from "@/lib/wheel-input";
import {
  clampDollyFactor,
  cursorAnchor,
  distanceRange,
  dollyAbout,
  panOffset,
  worldPerPixel,
} from "./orbit-input";

/** ボタンからの拡大・縮小の依頼（押した回数 n と、1 回の拡大率）。n が増えるたびに画面の中心へ向けて 1 回動かす */
export interface ZoomRequest {
  n: number;
  factor: number;
}

/** 視点の操作（`FreeOrbitControls`。`useThree` の controls に入る）のうち使う部分 */
interface Orbit {
  target: Vector3;
  update: () => void;
}

/** 注視点を中心に（cursor があればそのカーソルの下の点を中心に）factor 倍に拡大する */
function zoomView(
  camera: PerspectiveCamera,
  orbit: Orbit,
  factor: number,
  ndc: { x: number; y: number } | null,
) {
  const distance = camera.position.distanceTo(orbit.target);
  const f = clampDollyFactor(distance, factor, distanceRange(camera.near, camera.far));
  const right = new Vector3().setFromMatrixColumn(camera.matrixWorld, 0);
  const up = new Vector3().setFromMatrixColumn(camera.matrixWorld, 1);
  const h = distance * Math.tan((camera.fov * Math.PI) / 360);
  const anchor = ndc
    ? cursorAnchor(orbit.target, right, up, ndc, { w: h * camera.aspect, h })
    : orbit.target.clone();
  dollyAbout(camera.position, orbit.target, anchor, f);
  orbit.update();
}

/**
 * 3D のホイール入力を受ける（lib/wheel-input.ts で読み分ける）。ホイールは視点の操作（`FreeOrbitControls`）が読まず、ここで扱う: scroll（トラックパッドの 2 本指）は画面に平行にカメラと注視点を動かし、
 * notch（マウスのホイール、Ctrl / ⌘ 付きも）と pinch は注視点の奥行きの面の上のカーソルの下の点を中心に拡大する。
 * 自分で拡大するのは、目盛り 1 つの拡大率とピンチの感度を図と揃え、カーソルを中心にするため。
 * この上のホイールではページを動かさず、ピンチでブラウザの拡大も起こさない。
 * ボタンの依頼（`request`）も同じ拡大で扱う。frameloop="demand" なので、動かしたら描き直しを依頼する。
 */
export function ViewInput3D({ request }: { request: ZoomRequest | null }) {
  const camera = useThree((s) => s.camera);
  const gl = useThree((s) => s.gl);
  const controls = useThree((s) => s.controls) as Orbit | null;
  const invalidate = useThree((s) => s.invalidate);
  const done = useRef(request?.n ?? 0);

  useEffect(() => {
    if (!controls || !(camera instanceof PerspectiveCamera)) return;
    const el = gl.domElement;
    const classify = createWheelClassifier();
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      // ほかの要素のホイールの処理（ページのスクロールなど）へは渡さない
      e.stopImmediatePropagation();
      const kind = classify(e, e.timeStamp);
      const rect = el.getBoundingClientRect();
      camera.updateMatrixWorld();
      if (kind === "scroll") {
        const { x, y } = wheelPixels(e, rect.height);
        const d = camera.position.distanceTo(controls.target);
        const right = new Vector3().setFromMatrixColumn(camera.matrixWorld, 0);
        const up = new Vector3().setFromMatrixColumn(camera.matrixWorld, 1);
        const move = panOffset(x, y, worldPerPixel(d, camera.fov, rect.height), right, up);
        camera.position.add(move);
        controls.target.add(move);
        controls.update();
      } else {
        const ndc = {
          x: ((e.clientX - rect.left) / rect.width) * 2 - 1,
          y: -(((e.clientY - rect.top) / rect.height) * 2 - 1),
        };
        // 1 回の拡大は目盛り 1 段・ピンチ 1 回分に収め、飛びすぎないようにする
        const factor = clampZoom(zoomFactor(kind, e.deltaY), 0.5, 2);
        zoomView(camera, controls, factor, ndc);
      }
      invalidate();
    };
    // passive: false でないと preventDefault できない（ページのスクロールとブラウザの拡大を止める）
    // capture: 親の要素より先に受ける
    el.addEventListener("wheel", onWheel, { capture: true, passive: false });
    return () => el.removeEventListener("wheel", onWheel, { capture: true });
  }, [camera, gl, controls, invalidate]);

  useEffect(() => {
    if (!request || request.n === done.current) return;
    done.current = request.n;
    if (!controls || !(camera instanceof PerspectiveCamera)) return;
    camera.updateMatrixWorld();
    zoomView(camera, controls, request.factor, null);
    invalidate();
  }, [request, camera, controls, invalidate]);
  return null;
}
