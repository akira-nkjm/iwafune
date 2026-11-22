/**
 * ホイール入力の読み方（図と 3D で共通。ここだけが「どの操作か」を決める）。
 * トラックパッドとマウスは同じ wheel イベントで届くので、量と向きから 3 つに分ける。
 * - pinch: トラックパッドのピンチ（ブラウザは Ctrl 付きの小さな量で送る）
 * - notch: マウスホイールの 1 目盛り（行単位、または 1 回の量が大きい縦だけの動き。修飾キーの有無は問わない）
 * - scroll: トラックパッドの 2 本指のスクロール（小さな量。横が混じることもある）
 */

/** wheel イベントのうち読む項目（DOM に依らずテストできるよう、型だけ取り出す） */
export interface WheelLike {
  ctrlKey: boolean;
  metaKey: boolean;
  /** 0: ピクセル、1: 行、2: ページ */
  deltaMode: number;
  deltaX: number;
  deltaY: number;
}

export type WheelKind = "pinch" | "notch" | "scroll";

/** ピクセル単位で、縦だけの動きがこれ以上なら 1 目盛りとみなす [px]（Chromium 100・Windows 120・Firefox 約 53 を拾い、トラックパッドの普通の量は拾わない） */
export const NOTCH_MIN_DELTA = 40;
/** マウスホイール 1 目盛りの拡大率（拡大は ×、縮小は ÷） */
export const NOTCH_ZOOM_STEP = 1.2;
/** ピンチの感度。拡大率は exp(−deltaY × この値) */
export const PINCH_ZOOM_RATE = 0.01;
/** 行単位の量をピクセルに直す [px / 行] */
export const LINE_PX = 16;
/** 直前にスクロールがあってからこの間の大きな縦の量は、目盛りでなくスクロールの続き（速く払ったトラックパッド）とみなす [ms] */
export const SCROLL_HOLD_MS = 120;

/** イベント 1 つを分ける（状態を持たない） */
export function classifyWheel(e: WheelLike): WheelKind {
  if (e.deltaMode !== 0 && e.deltaY !== 0) return "notch";
  if (e.deltaX === 0 && Math.abs(e.deltaY) >= NOTCH_MIN_DELTA) return "notch";
  if (e.ctrlKey || e.metaKey) return "pinch";
  return "scroll";
}

/**
 * 分類器を作る。`classifyWheel` に加えて、修飾キー無しで直前（SCROLL_HOLD_MS 以内）にスクロールがあったときの
 * 大きな縦の量は scroll に直す（トラックパッドを速く払うと 1 回が NOTCH_MIN_DELTA を超えて、拡大に化けないように）。
 * `at` は event.timeStamp [ms]。
 */
export function createWheelClassifier(): (e: WheelLike, at: number) => WheelKind {
  let lastScroll = Number.NEGATIVE_INFINITY;
  return (e, at) => {
    let kind = classifyWheel(e);
    if (kind === "notch" && !(e.ctrlKey || e.metaKey) && e.deltaMode === 0) {
      if (at - lastScroll < SCROLL_HOLD_MS) kind = "scroll";
    }
    if (kind === "scroll") lastScroll = at;
    return kind;
  };
}

/** 1 回のイベントの拡大率（>1 で拡大）。notch は一定の段、pinch は量に比例（指を開く = deltaY が負 = 拡大） */
export function zoomFactor(kind: "pinch" | "notch", deltaY: number): number {
  if (kind === "notch") return deltaY < 0 ? NOTCH_ZOOM_STEP : 1 / NOTCH_ZOOM_STEP;
  return Math.exp(-deltaY * PINCH_ZOOM_RATE);
}

/** 拡大率を範囲に収める */
export function clampZoom(zoom: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, zoom));
}

/** スクロールの量をピクセルにそろえる（行・ページ単位を直す） */
export function wheelPixels(e: Pick<WheelLike, "deltaMode" | "deltaX" | "deltaY">, pagePx: number) {
  const k = e.deltaMode === 1 ? LINE_PX : e.deltaMode === 2 ? pagePx : 1;
  return { x: e.deltaX * k, y: e.deltaY * k };
}
