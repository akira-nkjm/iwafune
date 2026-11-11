import { createContext, useContext } from "react";
import { queryClient } from "@/api/queries";
import type { NavItemKey } from "@/lib/nav";
import { type LazyPage, quietly } from "./lazy-page";

/**
 * 先読みの計画。何を読むか（ページの本体・系の解析・大きなライブラリ）はページの登録（`routes/pages.ts`）が決めて
 * `Frame` に渡す。shell はページの中身（features）を知らない。
 */
export interface PreloadPlan {
  /** ナビの項目にホバー・フォーカスしたときに読むもの */
  item: (key: NavItemKey) => void;
  /** 最初のページが落ち着いたあとの空き時間に読む、軽いページの本体 */
  idle: readonly LazyPage[];
}

const NO_PRELOAD: PreloadPlan = { item: () => undefined, idle: [] };

/** `Frame` が渡した先読みの計画（ナビの項目が使う） */
export const PreloadContext = createContext<PreloadPlan>(NO_PRELOAD);

export function usePreloadPlan(): PreloadPlan {
  return useContext(PreloadContext);
}

/** ホバーとみなすまでの待ち [ms]（サイドバーの上をなぞっただけでは解析を頼まない） */
const HOVER_MS = 80;

/** ホバーは同時に 1 つの項目にしかないので、待ちのタイマーも 1 つ */
let hoverTimer: ReturnType<typeof setTimeout> | undefined;

function cancelHover(): void {
  if (hoverTimer !== undefined) clearTimeout(hoverTimer);
  hoverTimer = undefined;
}

/** ナビのリンクに付ける先読みのハンドラ（ホバーが続いたら・フォーカスしたら・押し始めたら `run` を呼ぶ） */
export function preloadHandlers(run: () => void) {
  const now = () => {
    cancelHover();
    run();
  };
  return {
    onPointerEnter: () => {
      cancelHover();
      hoverTimer = setTimeout(now, HOVER_MS);
    },
    onPointerLeave: cancelHover,
    onFocus: now,
    onPointerDown: now,
  };
}

/** データを取り終えたかを見る間隔 [ms] と、待つ回数の上限（取り終えなくても 10 秒であきらめて読む） */
const QUIET_POLL_MS = 250;
const QUIET_TRIES = 40;

/**
 * 最初のページが落ち着いたあとに、軽いページの本体を先に読んでおく。
 * 最初のページのデータ（設計・解析）を取り終えてから、ブラウザの空き時間に読む（回線と CPU を取り合わないように）。
 * 戻り値は取りやめる関数（`useEffect` の片付け）。
 */
export function preloadIdlePages(pages: readonly LazyPage[]): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let idle: number | undefined;
  const run = () => {
    for (const p of pages) quietly(p.load());
  };
  const whenQuiet = (tries: number) => {
    if (queryClient.isFetching() > 0 && tries > 0) {
      timer = setTimeout(() => whenQuiet(tries - 1), QUIET_POLL_MS);
    } else if (typeof window.requestIdleCallback === "function") {
      idle = window.requestIdleCallback(run, { timeout: 2000 });
    } else {
      timer = setTimeout(run, 0);
    }
  };
  timer = setTimeout(() => whenQuiet(QUIET_TRIES), QUIET_POLL_MS);
  return () => {
    clearTimeout(timer);
    if (idle !== undefined) window.cancelIdleCallback(idle);
  };
}
