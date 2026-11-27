/**
 * 画面が落ち着くまでの待ち方（読み込み中の表示がなく、API の通信がなく、本文が出ている）と、3D の描画の待ち方。
 */
import type { Locator, Page } from "playwright-core";

/** 読み込み中の表示（GUI の skeleton と、作業中を示す表示）。 */
const BUSY = "[data-slot=skeleton],[role=status][aria-busy=true]";
/** 読み込み中の文言（英語・日本語）。数値の欄などが文言のまま残っていないことを確かめる。 */
const LOADING_TEXT = /Loading…|読み込み中…/;

/** ページの `/api/` の通信の数を数える（`settle` が通信の終わりを待つ）。 */
export interface ApiTracker {
  pending(): number;
}

/** `/api/` の通信の数え始め。ページを開く前に呼ぶ。 */
export function trackApi(page: Page): ApiTracker {
  const open = new Set<unknown>();
  const isApi = (url: string) => new URL(url).pathname.startsWith("/api/");
  page.on("request", (r) => {
    if (isApi(r.url())) open.add(r);
  });
  page.on("requestfinished", (r) => open.delete(r));
  page.on("requestfailed", (r) => open.delete(r));
  return { pending: () => open.size };
}

/** 画面が落ち着くまで待つ。時間内に落ち着かなければ、写真の名前と理由を添えた誤りにする。 */
export async function settle(
  page: Page,
  api: ApiTracker,
  label: string,
  timeoutMs = 60000,
): Promise<void> {
  const started = Date.now();
  let stable = 0;
  let last = "";
  while (Date.now() - started < timeoutMs) {
    const state = await page.evaluate(
      ([busy, loading]) => {
        const main = document.querySelector("main, [role=main], #root");
        const text = main?.textContent ?? "";
        return {
          busy: document.querySelectorAll(busy as string).length,
          loading: new RegExp(loading as string).test(text),
          text: text.length,
        };
      },
      [BUSY, LOADING_TEXT.source],
    );
    const pending = api.pending();
    last = `busy=${state.busy}, loading text=${state.loading}, pending API calls=${pending}, text length=${state.text}`;
    if (state.busy === 0 && !state.loading && pending === 0 && state.text > 20) {
      stable += 1;
      if (stable >= 3) {
        await page.evaluate(() => document.fonts.ready);
        return;
      }
    } else {
      stable = 0;
    }
    await page.waitForTimeout(250);
  }
  throw new Error(`shot ${label}: the page did not settle within ${timeoutMs / 1000} s (${last})`);
}

/**
 * 3D の canvas が描かれるまで待つ（GPU の無い環境では SwiftShader で描くので時間がかかる）。
 * canvas を撮った画像が、無地の画像より十分大きくなるまで（描かれた図形があるまで）繰り返す。
 */
export async function waitForCanvas(
  page: Page,
  label: string,
  timeoutMs = 60000,
  minBytes = 12000,
): Promise<void> {
  const canvas: Locator = page.locator("canvas").first();
  await canvas.waitFor({ state: "visible", timeout: timeoutMs });
  const started = Date.now();
  let size = 0;
  while (Date.now() - started < timeoutMs) {
    size = (await canvas.screenshot()).length;
    if (size >= minBytes) return;
    await page.waitForTimeout(500);
  }
  throw new Error(
    `shot ${label}: the 3D canvas stayed blank for ${timeoutMs / 1000} s (${size} bytes)`,
  );
}
