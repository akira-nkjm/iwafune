import { useThree } from "@react-three/fiber";
import { useEffect, useMemo, useState } from "react";
import { CanvasTexture, Quaternion, SRGBColorSpace, Vector3 } from "three";
import { paletteFor, type Scheme } from "@/lib/tone";
import { CUBE_VIEWS, type CubeView, lookQuaternion } from "./view-cube";

/** 立方体の一辺の長さ [px]（GizmoHelper の画面の隅の座標系は 1 単位 = 1 px） */
const SIZE = 60;
/** 辺・角の区画の中心の位置（立方体の一辺を 1 とした大きさ）。面の区画はその内側の 0.5 四方 */
const OFFSET = 0.38;
/** 辺・角の区画の幅。面の上の辺は面から 0.005 だけ出して、面より先に当たるようにする */
const THICK = 0.25;
const LONG = 0.5;
const TEXTURE_PX = 128;

type Palette = ReturnType<typeof paletteFor>;

/** 面の貼り絵: 地・枠・ラベル。ホバー中は強調の色にする。ラベルは面の上の向きが画面の上になる向きで描く */
function faceTexture(label: string, p: Palette, hover: boolean): CanvasTexture {
  const canvas = document.createElement("canvas");
  canvas.width = TEXTURE_PX;
  canvas.height = TEXTURE_PX;
  const g = canvas.getContext("2d");
  if (g) {
    g.fillStyle = hover ? p.accent : p.chip;
    g.fillRect(0, 0, TEXTURE_PX, TEXTURE_PX);
    g.strokeStyle = hover ? p["accent-ink"] : p["ink-4"];
    g.lineWidth = 6;
    g.strokeRect(0, 0, TEXTURE_PX, TEXTURE_PX);
    g.fillStyle = hover ? p.inverse : p.ink;
    g.font = '600 46px "Noto Sans Mono", monospace';
    g.textAlign = "center";
    g.textBaseline = "middle";
    g.fillText(label, TEXTURE_PX / 2, TEXTURE_PX / 2 + 3);
  }
  const texture = new CanvasTexture(canvas);
  texture.colorSpace = SRGBColorSpace;
  return texture;
}

/** 区画の位置・大きさ・面の向き（面は 1 枚の板。辺と角は面に重ねる小さな箱で、ホバーしたときだけ色を付ける） */
function placement(v: CubeView) {
  if (v.kind === "face") {
    const q = lookQuaternion(v.direction, v.up);
    return { position: new Vector3(...v.sign).multiplyScalar(0.5), quaternion: q, size: null };
  }
  const size = v.sign.map((s) => (s === 0 ? LONG : THICK)) as [number, number, number];
  return {
    position: new Vector3(...v.sign).multiplyScalar(OFFSET),
    quaternion: new Quaternion(),
    size,
  };
}

/**
 * ビューキューブ（`GizmoHelper` の中に置く。カメラに合わせて回る）。面は PX・MX・PY・MY・PZ・MZ と書いた 6 枚の板、
 * 辺 12・角 8 は面のふちに重ねた見えない箱で、合わせて 26 の区画を押せる。ホバーした区画は強調の色になり、押すと `onPick` を呼ぶ
 * （向きの表は `view-cube.ts`。カメラを動かすのは呼び出し側）。色は配色（`scheme`）のパレットから引く。
 * `label` は区画に載せる説明（ツールチップ）。
 */
export function ViewCube({
  scheme,
  onPick,
  label,
}: {
  scheme: Scheme;
  onPick: (view: CubeView) => void;
  label: (name: string) => string;
}) {
  const gl = useThree((s) => s.gl);
  const p = paletteFor(scheme);
  const [hover, setHover] = useState<string | null>(null);
  const faces = useMemo(
    () =>
      CUBE_VIEWS.filter((v) => v.kind === "face").map((v) => ({
        view: v,
        normal: faceTexture(v.name, p, false),
        lit: faceTexture(v.name, p, true),
        ...placement(v),
      })),
    [p],
  );
  useEffect(
    () => () => {
      for (const f of faces) {
        f.normal.dispose();
        f.lit.dispose();
      }
    },
    [faces],
  );
  const blocks = useMemo(
    () => CUBE_VIEWS.filter((v) => v.kind !== "face").map((v) => ({ view: v, ...placement(v) })),
    [],
  );
  // 押せる所にカーソルを載せたら手の形にし、説明（ツールチップ）を出す
  useEffect(() => {
    const el = gl.domElement;
    el.style.cursor = hover ? "pointer" : "";
    el.title = hover ? label(hover) : "";
    return () => {
      el.style.cursor = "";
      el.title = "";
    };
  }, [gl, hover, label]);
  const enter = (name: string) => (e: { stopPropagation: () => void }) => {
    e.stopPropagation();
    setHover(name);
  };
  const leave = (name: string) => (e: { stopPropagation: () => void }) => {
    e.stopPropagation();
    setHover((h) => (h === name ? null : h));
  };
  const click = (view: CubeView) => (e: { stopPropagation: () => void }) => {
    e.stopPropagation();
    onPick(view);
  };
  return (
    <group scale={SIZE}>
      {faces.map((f) => (
        // biome-ignore lint/a11y/noStaticElementInteractions: three.js の mesh（DOM の要素ではない）
        <mesh
          key={f.view.name}
          position={f.position}
          quaternion={f.quaternion}
          onPointerOver={enter(f.view.name)}
          onPointerOut={leave(f.view.name)}
          onClick={click(f.view)}
        >
          <planeGeometry args={[1, 1]} />
          <meshBasicMaterial map={hover === f.view.name ? f.lit : f.normal} />
        </mesh>
      ))}
      {blocks.map((b) => (
        // biome-ignore lint/a11y/noStaticElementInteractions: three.js の mesh（DOM の要素ではない）
        <mesh
          key={b.view.name}
          position={b.position}
          scale={1.01}
          onPointerOver={enter(b.view.name)}
          onPointerOut={leave(b.view.name)}
          onClick={click(b.view)}
        >
          <boxGeometry args={b.size ?? [1, 1, 1]} />
          <meshBasicMaterial
            color={p.accent}
            transparent
            opacity={0.75}
            visible={hover === b.view.name}
          />
        </mesh>
      ))}
    </group>
  );
}
