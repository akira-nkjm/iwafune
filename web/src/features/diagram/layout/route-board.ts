import { type Box, GRID, type Pt, snapDown, snapUp } from "./geometry";

/**
 * 線の経路探索（route.ts）の盤面と作業領域。盤面は GRID の格子点ごとの、通れない所（カードの周り・
 * 枠の見出し）、いちばん内側の枠、枠の縁、線の向きごとの通っているネット。作業領域は A* の表と最小ヒープ。
 */

/** 図の外周の余白（格子の数） */
const OUTER = 10;
/**
 * 線とカードの縁のすき間の最小 [px]。カードの横を通る線は縁からこれだけ離す（近すぎると縁と見分けにくい）。
 * 端子の出口（縁から 1 格子の所）だけは、その端子の線が通る。
 */
export const CLEARANCE = 2 * GRID;

export interface Frame {
  id: string;
  box: Box;
  /** 枠の中で線を通さない所（見出しの文字） */
  title: Box;
  depth: number;
}

export class Board {
  readonly x0: number;
  readonly y0: number;
  readonly nx: number;
  readonly ny: number;
  readonly blocked: Uint8Array;
  /** いちばん内側の枠の番号 + 1（0 は枠の外） */
  readonly frame: Int16Array;
  /** 枠の縁とその両隣（1: 横の縁、2: 縦の縁） */
  readonly border: Uint8Array;
  /** 線の向きごとの、通っているネットの番号 + 1（0 は空き、-1 は複数のネット） */
  readonly occ: [Int32Array, Int32Array];

  constructor(cards: Box[], frames: Frame[], extra: Pt[]) {
    const xs = [...cards, ...frames.map((f) => f.box)].flatMap((b) => [b.x, b.x + b.width]);
    const ys = [...cards, ...frames.map((f) => f.box)].flatMap((b) => [b.y, b.y + b.height]);
    xs.push(...extra.map((p) => p.x));
    ys.push(...extra.map((p) => p.y));
    this.x0 = snapDown(Math.min(0, ...xs)) - OUTER * GRID;
    this.y0 = snapDown(Math.min(0, ...ys)) - OUTER * GRID;
    this.nx = (snapUp(Math.max(0, ...xs)) - this.x0) / GRID + OUTER + 1;
    this.ny = (snapUp(Math.max(0, ...ys)) - this.y0) / GRID + OUTER + 1;
    const n = this.nx * this.ny;
    this.blocked = new Uint8Array(n);
    this.frame = new Int16Array(n);
    this.border = new Uint8Array(n);
    this.occ = [new Int32Array(n), new Int32Array(n)];
    // カード: 縁から CLEARANCE 未満の所は通さない
    for (const c of cards) this.fill(c, CLEARANCE - 1, (i) => (this.blocked[i] = 1));
    frames
      .toSorted((a, b) => a.depth - b.depth)
      .forEach((f, k) => {
        this.fill(f.box, 0, (i) => (this.frame[i] = k + 1));
        this.fill(f.title, 0, (i) => (this.blocked[i] = 1));
        this.edges(f.box);
      });
  }

  /** 箱（`pad` だけ広げた内側。縁ちょうどは含まない）に入る格子点ごとに `f` を呼ぶ */
  private fill(b: Box, pad: number, f: (i: number) => void) {
    const gx0 = Math.ceil((b.x - pad - this.x0) / GRID + 1e-9);
    const gx1 = Math.floor((b.x + b.width + pad - this.x0) / GRID - 1e-9);
    const gy0 = Math.ceil((b.y - pad - this.y0) / GRID + 1e-9);
    const gy1 = Math.floor((b.y + b.height + pad - this.y0) / GRID - 1e-9);
    for (let gy = Math.max(0, gy0); gy <= Math.min(this.ny - 1, gy1); gy++)
      for (let gx = Math.max(0, gx0); gx <= Math.min(this.nx - 1, gx1); gx++) f(gy * this.nx + gx);
  }

  /**
   * 枠の縁と、その内と外の 1 格子に印を付ける（枠の位置と大きさは GRID の倍数）。縁に沿って走る線は縁と
   * 見分けにくいので、平行に走るときだけ費用を足す（横切るのは構わない）
   */
  private edges(b: Box) {
    const [gx0, gy0] = this.cell({ x: b.x, y: b.y });
    const [gx1, gy1] = this.cell({ x: b.x + b.width, y: b.y + b.height });
    const near = (v0: number, v1: number) => [v0 - 1, v0, v0 + 1, v1 - 1, v1, v1 + 1];
    for (let gx = gx0 - 1; gx <= gx1 + 1; gx++)
      for (const gy of near(gy0, gy1)) this.flag(gx, gy, 1);
    for (let gy = gy0 - 1; gy <= gy1 + 1; gy++)
      for (const gx of near(gx0, gx1)) this.flag(gx, gy, 2);
  }

  private flag(gx: number, gy: number, bit: number) {
    if (!this.inside(gx, gy)) return;
    const i = gy * this.nx + gx;
    this.border[i] = (this.border[i] ?? 0) | bit;
  }

  inside(gx: number, gy: number) {
    return gx >= 0 && gy >= 0 && gx < this.nx && gy < this.ny;
  }
  cell(p: Pt): [number, number] {
    return [Math.round((p.x - this.x0) / GRID), Math.round((p.y - this.y0) / GRID)];
  }
  point(i: number): Pt {
    return { x: this.x0 + (i % this.nx) * GRID, y: this.y0 + Math.floor(i / this.nx) * GRID };
  }
  mark(axis: 0 | 1, i: number, net: number) {
    const o = this.occ[axis][i] ?? 0;
    this.occ[axis][i] = o === 0 || o === net + 1 ? net + 1 : -1;
  }
  clear() {
    this.occ[0].fill(0);
    this.occ[1].fill(0);
  }
}

/** 探索の作業領域（線ごとに確保しないよう使い回す） */
export class Work {
  g = new Float32Array(0);
  parent = new Int32Array(0);
  seen = new Uint32Array(0);
  closed = new Uint32Array(0);
  stamp = 0;
  readonly heap = new Heap();
  ensure(n: number) {
    if (this.g.length >= n) return;
    this.g = new Float32Array(n);
    this.parent = new Int32Array(n);
    this.seen = new Uint32Array(n);
    this.closed = new Uint32Array(n);
    this.stamp = 0;
  }
}

/**
 * 最小ヒープ（費用と状態）。型付き配列に持ち、探索ごとに作り直さず `clear` で使い回す。
 * 同じ費用の要素を取り出す順は、押す順と下ろし方だけで決まる（配列で持っていた頃と同じ）。
 */
export class Heap {
  private keys = new Float64Array(4096);
  private vals = new Int32Array(4096);
  size = 0;
  clear() {
    this.size = 0;
  }
  push(k: number, v: number) {
    if (this.size === this.keys.length) {
      const keys = new Float64Array(this.size * 2);
      const vals = new Int32Array(this.size * 2);
      keys.set(this.keys);
      vals.set(this.vals);
      this.keys = keys;
      this.vals = vals;
    }
    const keys = this.keys;
    const vals = this.vals;
    let i = this.size++;
    while (i > 0) {
      const p = (i - 1) >> 1;
      const pk = keys[p] ?? 0;
      if (pk <= k) break;
      keys[i] = pk;
      vals[i] = vals[p] ?? 0;
      i = p;
    }
    keys[i] = k;
    vals[i] = v;
  }
  pop(): number {
    const keys = this.keys;
    const vals = this.vals;
    const top = vals[0] ?? -1;
    const n = --this.size;
    const k = keys[n] ?? 0;
    const v = vals[n] ?? 0;
    if (n > 0) {
      let i = 0;
      for (;;) {
        const l = 2 * i + 1;
        if (l >= n) break;
        const r = l + 1;
        const c = r < n && (keys[r] ?? 0) < (keys[l] ?? 0) ? r : l;
        const ck = keys[c] ?? 0;
        if (ck >= k) break;
        keys[i] = ck;
        vals[i] = vals[c] ?? 0;
        i = c;
      }
      keys[i] = k;
      vals[i] = v;
    }
    return top;
  }
}
