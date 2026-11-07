import { prefetchSystemAnalyses, queryClient } from "@/api/queries";
import { loadLayout3D } from "@/features/layout3d/LazyLayout3D";
import { isSystem } from "@/features/systems/systems";
import type { NavItemKey } from "@/lib/nav";
import { quietly } from "./lazy-page";
import { pages } from "./pages";

/**
 * ナビの項目にホバー・フォーカスしたときに、そのページの本体と、時間のかかるデータ（系の解析）を先に読む。
 * 構造と熱は 3D 配置（three.js）も読む。
 */
export function preloadPage(key: NavItemKey): void {
  if (isSystem(key)) {
    quietly(pages.system.load());
    quietly(prefetchSystemAnalyses(key));
    if (key === "structure" || key === "thermal") quietly(loadLayout3D());
    return;
  }
  quietly(pages[key].load());
}

/** ホバーとみなすまでの待ち [ms]（サイドバーの上をなぞっただけでは解析を頼まない） */
const HOVER_MS = 80;

/** ホバーは同時に 1 つの項目にしかないので、待ちのタイマーも 1 つ */
let hoverTimer: ReturnType<typeof setTimeout> | undefined;

function cancelHover(): void {
  if (hoverTimer !== undefined) clearTimeout(hoverTimer);
  hoverTimer = undefined;
}

/** ナビのリンクに付ける先読みのハンドラ（ホバーが続いたら・フォーカスしたら・押し始めたら読む） */
export function preloadHandlers(key: NavItemKey) {
  const now = () => {
    cancelHover();
    preloadPage(key);
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

/** 大きなチャンク（系統図の React Flow、3D の three.js）は空き時間には読まない。ホバーで読む */
const IDLE: readonly (keyof typeof pages)[] = [
  "overview",
  "components",
  "catalog",
  "modes",
  "defs",
  "system",
  "orbit",
  "cases",
  "diagnostics",
  "settings",
];

/** データを取り終えたかを見る間隔 [ms] と、待つ回数の上限（取り終えなくても 10 秒であきらめて読む） */
const QUIET_POLL_MS = 250;
const QUIET_TRIES = 40;

/**
 * 最初のページが落ち着いたあとに、軽いページの本体を先に読んでおく。
 * 最初のページのデータ（設計・解析）を取り終えてから、ブラウザの空き時間に読む（回線と CPU を取り合わないように）。
 */
export function preloadIdlePages(): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let idle: number | undefined;
  const run = () => {
    for (const k of IDLE) quietly(pages[k].load());
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
