/**
 * ブラウザの描画の違い（表示の工夫を出し分けるところだけ。機能の有無はブラウザの API で調べる）。
 */

/**
 * `will-change: transform` で 1 枚の絵にした要素を、外したあとも絵にした時の解像度のまま引き伸ばすか（Safari などの
 * WebKit）。iOS のブラウザはどれも WebKit で描く。Chromium 系（Chrome・Edge・Opera）は外せば描き直す。
 */
export function keepsPromotedLayerResolution(userAgent: string = navigator.userAgent): boolean {
  if (/iPhone|iPad|iPod/.test(userAgent)) return true;
  return /AppleWebKit/.test(userAgent) && !/Chrome|Chromium|Edg|OPR/.test(userAgent);
}
