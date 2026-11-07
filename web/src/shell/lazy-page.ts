import type { ComponentType } from "react";

/** 遅れて読むページの本体（ページごとに別のチャンク） */
export interface LazyPage {
  /** 本体を読む。2 回目からは同じ約束を返す（失敗したときは次の呼び出しで読み直す） */
  load: () => Promise<ComponentType>;
  /** 読み終わっていれば本体（ルートはこれがあれば待たずに描く） */
  readonly current: ComponentType | null;
}

/** 名前付きの export を持つモジュールから、遅れて読むページを作る */
export function lazyPage<M>(importer: () => Promise<M>, pick: (m: M) => ComponentType): LazyPage {
  let current: ComponentType | null = null;
  let pending: Promise<ComponentType> | null = null;
  const load = () => {
    if (!pending) {
      pending = importer().then(
        (m) => {
          current = pick(m);
          return current;
        },
        (e: unknown) => {
          pending = null;
          throw e;
        },
      );
    }
    return pending;
  };
  return {
    load,
    get current() {
      return current;
    },
  };
}

/** 先読みの失敗は捨てる（ページを開いたときに LazyRoute がもう一度読み、エラーを出す） */
export function quietly(p: Promise<unknown>): void {
  p.catch(() => undefined);
}

/**
 * ルートの `clientLoader` にする関数。ルートのモジュールが読まれた時点でページの本体を読み始める
 * （描画を待たない。最初の読み込みで、ルート → 本体の 2 段の待ちを縮める）。何も待たずに null を返す。
 */
export function loadOnNavigate(page: LazyPage): () => null {
  return () => {
    quietly(page.load());
    return null;
  };
}
