import {
  createContext,
  type KeyboardEvent,
  type ReactNode,
  type RefCallback,
  useCallback,
  useContext,
  useState,
} from "react";

/** 行列の中のセルの位置（行は表の中の部品の通し番号、列はモードの順。`matrixGroups` が振る） */
export interface CellAt {
  row: number;
  column: number;
}

const ARROWS = {
  ArrowLeft: [0, -1],
  ArrowRight: [0, 1],
  ArrowUp: [-1, 0],
  ArrowDown: [1, 0],
} as const satisfies Record<string, readonly [number, number]>;

/** 矢印キーで動いた先の位置。矢印でなければ null（端の外も位置は返す。そこにセルが無ければ動かない） */
export function arrowTarget(key: string, at: CellAt): CellAt | null {
  if (!(key in ARROWS)) return null;
  const [dr, dc] = ARROWS[key as keyof typeof ARROWS];
  return { row: at.row + dr, column: at.column + dc };
}

/** フォーカスを動かせるもの（セルのボタン） */
type Focusable = Pick<HTMLElement, "focus">;

/** 矢印キーの移動に使うキー（押されたキーと、既定の動き（スクロール）を止める口） */
type ArrowEvent = Pick<KeyboardEvent, "key" | "preventDefault">;

/** 行列のセルの要素を位置で覚え、矢印キーで隣のセルへフォーカスを動かすもの */
export interface CellFocus {
  /** セルの要素を覚える。戻り値で忘れる（ほかの要素が同じ位置に入っていれば消さない） */
  register: (at: CellAt, element: Focusable) => () => void;
  /** 矢印キーなら隣のセルへフォーカスを動かし、キーを使ったので true（端では動かない）。ほかのキーは false */
  move: (event: ArrowEvent, at: CellAt) => boolean;
}

const keyOf = (at: CellAt) => `${at.row}:${at.column}`;

/** 行列 1 つ分の `CellFocus`（`MatrixFocus` が 1 つ持つ。テストでは直接作る） */
export function cellFocus(): CellFocus {
  const cells = new Map<string, Focusable>();
  return {
    register: (at, element) => {
      const key = keyOf(at);
      cells.set(key, element);
      return () => {
        if (cells.get(key) === element) cells.delete(key);
      };
    },
    move: (event, at) => {
      const next = arrowTarget(event.key, at);
      if (!next) return false;
      event.preventDefault();
      cells.get(keyOf(next))?.focus();
      return true;
    },
  };
}

const MatrixFocusContext = createContext<CellFocus | null>(null);

/** 行列（`ModesTable`）がセルを包む。中のセルは `useCellFocus` で矢印キーの移動を受ける */
export function MatrixFocus({ children }: { children: ReactNode }) {
  const [focus] = useState(cellFocus);
  return <MatrixFocusContext.Provider value={focus}>{children}</MatrixFocusContext.Provider>;
}

/**
 * セルのボタンに付ける `ref` と、キーを受ける `onKeyDown`（矢印キーを使ったら true。ほかのキーはセルが扱う）。
 * どのセルへ動くかは行列の側（`MatrixFocus`）が決める。行列の外では何もしない。
 */
export function useCellFocus(at: CellAt): {
  ref: RefCallback<HTMLElement>;
  onKeyDown: (event: ArrowEvent) => boolean;
} {
  const focus = useContext(MatrixFocusContext);
  const { row, column } = at;
  const ref = useCallback<RefCallback<HTMLElement>>(
    (element) => (focus && element ? focus.register({ row, column }, element) : undefined),
    [focus, row, column],
  );
  return { ref, onKeyDown: (event) => focus?.move(event, { row, column }) ?? false };
}
