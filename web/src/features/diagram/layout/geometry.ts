/** 系統図の配置の寸法と型（layout.ts・layout-*.ts・grid.ts・表示の部品が共有する葉のモジュール）。 */

export const CARD_W = 150;
export const CARD_H = 52;
export const PORT = 6;
export const JUNCTION = 12;
/** 枠の見出し（サブシステム名）の帯の高さ [px]。線のラベルはここを避ける */
export const TITLE_H = 30;

/**
 * 配置と配線の格子の間隔 [px]。カード・枠・端子は格子の上に置き、線は格子の上を通る（route.ts）。
 * 端子の間隔と枠の余白もこの倍数にする。
 */
export const GRID = 8;
/** 端子どうしの間隔 [px]（同じ側に並ぶ端子） */
export const PORT_GAP = 2 * GRID;
export const snapUp = (v: number) => Math.ceil(v / GRID) * GRID;
export const snapDown = (v: number) => Math.floor(v / GRID) * GRID;

/** 端子を出すカードの側 */
export type Side = "left" | "right" | "top" | "bottom";

export interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Pt {
  x: number;
  y: number;
}

/**
 * 分かれ口: 線の ID → その線の端を、端子とは別の側に出す位置（カードの左上からの相対、端子の中心）。何本も線が出る
 * 端子（バス）で、相手が端子の反対側にいる線だけが持つ（ports.ts）。
 */
export type Taps = Map<string, { source?: Pt & { side: Side }; target?: Pt & { side: Side } }>;

export interface DiagramLayout {
  /** 親に対する相対位置（React Flow の position）と大きさ */
  boxes: Map<string, Box>;
  /** 絶対位置（電源ブロックの枠と辺の座標の換算に使う） */
  absolute: Map<string, Pt>;
  /** ポートの位置（カードの左上からの相対、ポートの中心） */
  ports: Map<string, Pt & { side: Side }>;
  /** 線の端を端子とは別の側に出す分かれ口（Taps） */
  taps: Taps;
  /** 辺の経路（絶対座標の折れ線） */
  routes: Map<string, Pt[]>;
  /** ラベルを描く辺と、その中心（絶対座標）。同じ端子から出る同じプロトコルの線は 1 本にだけ付ける */
  labels: Map<string, Pt>;
  /** 経路が見つからなかった線（候補を比べるときだけ使う。score.ts） */
  unrouted?: ReadonlySet<string>;
}

/** 並べた結果: 親からの相対の箱と、絶対位置 */
export interface Placement {
  boxes: Map<string, Box>;
  absolute: Map<string, Pt>;
}

/** 大きさ（幅と高さ） */
export type Size = { width: number; height: number };

/** 図の目標の縦横比（横 / 縦）。画面の作業領域に近い横長 */
export const TARGET_ASPECT = 16 / 10;

/** 縦横比が目標からどれだけ離れているか（比の対数の差。0 が目標どおり） */
export const aspectMiss = (width: number, height: number) =>
  height > 0 ? Math.abs(Math.log(width / height / TARGET_ASPECT)) : Number.POSITIVE_INFINITY;

/** いくつかの箱をまとめた外形（絶対座標） */
export function extentOf(ids: Iterable<string>, layout: DiagramLayout): Box {
  let x0 = Number.POSITIVE_INFINITY;
  let y0 = Number.POSITIVE_INFINITY;
  let x1 = Number.NEGATIVE_INFINITY;
  let y1 = Number.NEGATIVE_INFINITY;
  for (const id of ids) {
    const p = layout.absolute.get(id);
    const b = layout.boxes.get(id);
    if (!p || !b) continue;
    x0 = Math.min(x0, p.x);
    y0 = Math.min(y0, p.y);
    x1 = Math.max(x1, p.x + b.width);
    y1 = Math.max(y1, p.y + b.height);
  }
  return x0 > x1
    ? { x: 0, y: 0, width: 0, height: 0 }
    : { x: x0, y: y0, width: x1 - x0, height: y1 - y0 };
}

/**
 * 線がつながる点。React Flow は端子（大きさ PORT の丸）の外向きの縁に線をつなぐ（@xyflow/system の
 * getHandlePosition）ので、端子の中心から PORT / 2 だけ外へ出た所。自動配置の経路の端もここに合わせる
 * （合っていれば RoutedEdge が自動配置の経路をそのまま描く）。
 */
export function anchorOf(p: Pt, side: Side): Pt {
  const d = PORT / 2;
  return {
    left: { x: p.x - d, y: p.y },
    right: { x: p.x + d, y: p.y },
    top: { x: p.x, y: p.y - d },
    bottom: { x: p.x, y: p.y + d },
  }[side];
}

/** ジャンクションの線がつながる点（JunctionNode のハンドルは丸全体で、向きは右。左上からの相対） */
export const JUNCTION_ANCHOR: Pt = { x: JUNCTION, y: JUNCTION / 2 };
