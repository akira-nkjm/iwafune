import { PerspectiveCamera, Vector3 } from "three";
import { expect, it } from "vitest";
import { FreeOrbitControls } from "./free-orbit";

/** pointer イベントを受ける最小の要素（DOM なしで操作を流し込む） */
class FakeElement extends EventTarget {
  style: Record<string, string> = {};
  clientHeight = 600;
  setPointerCapture() {}
  releasePointerCapture() {}
  send(type: string, init: Partial<PointerEvent> & { clientX: number; clientY: number }) {
    const e = Object.assign(new Event(type, { cancelable: true }), {
      pointerId: 1,
      button: 0,
      shiftKey: false,
      ...init,
    });
    this.dispatchEvent(e);
  }
}

function setup() {
  const el = new FakeElement();
  const camera = new PerspectiveCamera(40, 1, 0.1, 100);
  camera.position.set(0, -5, 0);
  camera.lookAt(0, 0, 0);
  camera.updateMatrixWorld();
  const controls = new FreeOrbitControls(camera, el as unknown as HTMLElement);
  controls.connect();
  const events: string[] = [];
  for (const t of ["start", "end", "change"] as const)
    controls.addEventListener(t, () => events.push(t));
  return { el, camera, controls, events };
}

it("左ドラッグで回る（注視点との距離は保つ）。動き始めで start、離して end", () => {
  const { el, camera, controls, events } = setup();
  el.send("pointerdown", { clientX: 100, clientY: 100 });
  el.send("pointermove", { clientX: 100, clientY: 200 });
  el.send("pointerup", { clientX: 100, clientY: 200 });
  expect(events[0]).toBe("start");
  expect(events.at(-1)).toBe("end");
  expect(camera.position.distanceTo(controls.target)).toBeCloseTo(5, 9);
  expect(camera.position.z).toBeGreaterThan(0.5);
});

it("ごく小さな動きでは始めない（クリックと区別）", () => {
  const { el, camera, events } = setup();
  el.send("pointerdown", { clientX: 100, clientY: 100 });
  el.send("pointermove", { clientX: 101, clientY: 100 });
  el.send("pointerup", { clientX: 101, clientY: 100 });
  expect(events).toEqual([]);
  expect(camera.position.x).toBe(0);
});

it("Shift + 左ドラッグは移動: 注視点とカメラが一緒に動き、向きは変わらない", () => {
  const { el, camera, controls } = setup();
  const q = camera.quaternion.clone();
  el.send("pointerdown", { clientX: 100, clientY: 100, shiftKey: true });
  el.send("pointermove", { clientX: 160, clientY: 100, shiftKey: true });
  el.send("pointerup", { clientX: 160, clientY: 100 });
  expect(controls.target.x).toBeLessThan(0);
  expect(camera.position.x).toBeCloseTo(controls.target.x, 9);
  expect(camera.quaternion.angleTo(q)).toBeLessThan(1e-12);
});

it("enabled が false の間（部品を掴んでいる間）は視点を動かさない", () => {
  const { el, camera, controls } = setup();
  el.send("pointerdown", { clientX: 100, clientY: 100 });
  controls.enabled = false;
  el.send("pointermove", { clientX: 100, clientY: 300 });
  expect(camera.position.equals(new Vector3(0, -5, 0))).toBe(true);
  controls.enabled = true;
  el.send("pointermove", { clientX: 100, clientY: 400 });
  // 掴んで落とされた操作は、離すまで再開しない
  expect(camera.position.equals(new Vector3(0, -5, 0))).toBe(true);
});
