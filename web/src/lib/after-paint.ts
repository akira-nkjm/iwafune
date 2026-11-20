/**
 * いまの描画が画面に出たあとで `run` を呼ぶ（次のフレームの描画のあと）。重い計算や重い描画を始める前に、
 * 読み込み中の表示を先に画面へ出すために使う。これが無いと、計算が終わるまで前の画面のまま固まって見える。
 * 戻り値は取り消し（まだ呼ばれていなければ、呼ばない）。
 */
export function afterPaint(run: () => void): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  // requestAnimationFrame は描画の直前に呼ばれるので、そこからタイマーを 1 つはさんで描画のあとにする
  const frame = requestAnimationFrame(() => {
    timer = setTimeout(run, 0);
  });
  return () => {
    cancelAnimationFrame(frame);
    if (timer !== undefined) clearTimeout(timer);
  };
}
