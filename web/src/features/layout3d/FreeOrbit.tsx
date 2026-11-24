import { useThree } from "@react-three/fiber";
import { useEffect, useMemo } from "react";
import type { PerspectiveCamera } from "three";
import { FreeOrbitControls } from "./free-orbit";

/**
 * 視点の操作（`FreeOrbitControls`）を Canvas に入れる。drei の `makeDefault` と同じに、`useThree` の controls に登録する
 * （`Bounds`・`TransformControls`・`ViewInput3D` はここから読む）。動かすたびに描き直しを依頼する（frameloop="demand"）。
 */
export function FreeOrbit() {
  const camera = useThree((s) => s.camera);
  const gl = useThree((s) => s.gl);
  const invalidate = useThree((s) => s.invalidate);
  const get = useThree((s) => s.get);
  const set = useThree((s) => s.set);
  const controls = useMemo(
    () => new FreeOrbitControls(camera as PerspectiveCamera, gl.domElement),
    [camera, gl],
  );
  useEffect(() => {
    controls.connect();
    const onChange = () => invalidate();
    controls.addEventListener("change", onChange);
    const old = get().controls;
    set({ controls: controls as never });
    return () => {
      controls.removeEventListener("change", onChange);
      controls.dispose();
      set({ controls: old });
    };
  }, [controls, invalidate, get, set]);
  return null;
}
