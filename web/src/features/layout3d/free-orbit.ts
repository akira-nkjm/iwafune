import { EventDispatcher, type PerspectiveCamera, Vector3 } from "three";
import { radiansPerPixel, rotateBall } from "./free-rotation";
import {
  clampDollyFactor,
  distanceRange,
  dollyAbout,
  panOffset,
  worldPerPixel,
} from "./orbit-input";

/** 押してから、これだけ動くまでは視点の操作を始めない [px]（クリックと区別する） */
const START_SLOP_PX = 3;

type Mode = "rotate" | "pan" | "pinch";

interface Point {
  x: number;
  y: number;
}

/** 視点の操作が出す出来事（drei の `Bounds` が `start` で自分の動きを止める） */
interface OrbitEvents {
  start: object;
  end: object;
  change: object;
}

/**
 * 上の向きを固定しない視点の操作（OrbitControls の代わり）。左ドラッグ（タッチは 1 本指）= どの軸まわりにも回す（極で止まらない）、
 * 中・右ドラッグと Shift + 左ドラッグ = 画面に平行に移動、タッチの 2 本指 = 移動と拡大。ホイールは読まない（`ViewInput3D` が読む）。
 * `target`・`update`・`enabled` と `start`・`end`・`change` の出来事は OrbitControls と同じ形で、`useThree` の controls に入れると
 * drei の `Bounds`・`TransformControls`（掴んでいる間は `enabled` を落とす）がそのまま使える。
 * 向きは `camera.quaternion` だけが持つ（`camera.up` も `lookAt` も使わない）。
 */
export class FreeOrbitControls extends EventDispatcher<OrbitEvents> {
  /** 見ている点（カメラの回る中心） */
  readonly target = new Vector3();
  /** false の間は操作を受けない（部品を掴んでいる間は drei が落とす） */
  enabled = true;
  /** `Bounds` が書く（使わない） */
  maxDistance = Number.POSITIVE_INFINITY;

  private readonly pointers = new Map<number, Point>();
  private mode: Mode | null = null;
  private started = false;
  private origin: Point = { x: 0, y: 0 };
  private pinch = 0;

  private readonly camera: PerspectiveCamera;
  private readonly el: HTMLElement;

  constructor(camera: PerspectiveCamera, el: HTMLElement) {
    super();
    this.camera = camera;
    this.el = el;
  }

  /** 要素に操作をつなぐ */
  connect(): void {
    const el = this.el;
    el.addEventListener("pointerdown", this.onDown);
    el.addEventListener("pointermove", this.onMove);
    el.addEventListener("pointerup", this.onUp);
    el.addEventListener("pointercancel", this.onUp);
    el.addEventListener("contextmenu", this.onMenu);
    // pointer イベントで指の動きを受けるため、ブラウザの既定のスクロール・拡大を止める
    el.style.touchAction = "none";
  }

  /** つないだ操作を外す */
  dispose(): void {
    const el = this.el;
    el.removeEventListener("pointerdown", this.onDown);
    el.removeEventListener("pointermove", this.onMove);
    el.removeEventListener("pointerup", this.onUp);
    el.removeEventListener("pointercancel", this.onUp);
    el.removeEventListener("contextmenu", this.onMenu);
    el.style.touchAction = "";
  }

  /** 外から位置・注視点を動かしたあとに呼ぶ（描き直しの依頼を出すだけ。向きは変えない） */
  update(): void {
    this.dispatchEvent({ type: "change" });
  }

  private readonly onMenu = (e: Event) => e.preventDefault();

  private readonly onDown = (e: PointerEvent) => {
    if (!this.enabled) return;
    this.pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    try {
      this.el.setPointerCapture(e.pointerId);
    } catch {
      // 取れなくても（合成のイベントなど）操作は続けられる
    }
    this.origin = { x: e.clientX, y: e.clientY };
    this.started = false;
    if (this.pointers.size >= 2) {
      this.mode = "pinch";
      this.pinch = this.spread();
    } else if (e.button === 0 && !e.shiftKey) {
      this.mode = "rotate";
    } else {
      this.mode = "pan";
    }
  };

  private readonly onMove = (e: PointerEvent) => {
    const before = this.pointers.get(e.pointerId);
    if (!before || this.mode === null) return;
    // 部品を掴んだなら（TransformControls が enabled を落とす）、視点は動かさない
    if (!this.enabled) {
      this.finish();
      return;
    }
    const now = { x: e.clientX, y: e.clientY };
    this.pointers.set(e.pointerId, now);
    if (!this.started) {
      if (Math.hypot(now.x - this.origin.x, now.y - this.origin.y) < START_SLOP_PX) return;
      this.started = true;
      this.dispatchEvent({ type: "start" });
    }
    const dx = now.x - before.x;
    const dy = now.y - before.y;
    if (this.mode === "pinch") this.pinchStep(e.pointerId, dx, dy);
    else if (this.mode === "rotate") this.rotate(dx, dy);
    else this.pan(dx, dy);
    this.dispatchEvent({ type: "change" });
  };

  private readonly onUp = (e: PointerEvent) => {
    this.pointers.delete(e.pointerId);
    try {
      this.el.releasePointerCapture(e.pointerId);
    } catch {
      // 取っていなければ何もしない
    }
    if (this.pointers.size === 0) this.finish();
    else if (this.pointers.size === 1) {
      // 2 本指の片方を離したら、残りの指では何もしない（指の置き直しで回り出さないように）
      this.mode = null;
    }
  };

  private finish(): void {
    if (this.started) this.dispatchEvent({ type: "end" });
    this.started = false;
    this.mode = null;
    this.pointers.clear();
  }

  private height(): number {
    return this.el.clientHeight || 1;
  }

  private rotate(dx: number, dy: number): void {
    const c = this.camera;
    rotateBall(c.position, c.quaternion, this.target, dx, dy, radiansPerPixel(this.height()));
    c.updateMatrixWorld();
  }

  private pan(dx: number, dy: number): void {
    const c = this.camera;
    c.updateMatrixWorld();
    const right = new Vector3().setFromMatrixColumn(c.matrixWorld, 0);
    const up = new Vector3().setFromMatrixColumn(c.matrixWorld, 1);
    const per = worldPerPixel(c.position.distanceTo(this.target), c.fov, this.height());
    // 内容が指に付いてくる向き（指が右へ動けばカメラは左へ）
    const move = panOffset(-dx, -dy, per, right, up);
    c.position.add(move);
    this.target.add(move);
    c.updateMatrixWorld();
  }

  /** 2 本の指の間隔 [px] */
  private spread(): number {
    const [a, b] = [...this.pointers.values()];
    return a && b ? Math.hypot(a.x - b.x, a.y - b.y) : 0;
  }

  /** 2 本指: 動いた指の分だけ全体を移動し、指の間隔の比で拡大する（注視点を中心に） */
  private pinchStep(id: number, dx: number, dy: number): void {
    if (this.pointers.size < 2 || !this.pointers.has(id)) return;
    this.pan(dx / 2, dy / 2);
    const now = this.spread();
    if (this.pinch > 0 && now > 0) {
      const c = this.camera;
      const distance = c.position.distanceTo(this.target);
      const f = clampDollyFactor(distance, now / this.pinch, distanceRange(c.near, c.far));
      dollyAbout(c.position, this.target, this.target.clone(), f);
    }
    this.pinch = now;
  }
}
