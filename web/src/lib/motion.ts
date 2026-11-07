/**
 * 画面の部分を開け閉めする動きの時間と曲線（サイドバーの幅、保存の帯の高さなど）。立ち上がりが速く
 * 終わりが静かな ease-out。動きを減らす設定では動かさない。Tailwind の transition-* と組み合わせて使う。
 */
export const PANEL_MOTION =
  "duration-[260ms] ease-[cubic-bezier(0.2,0,0,1)] motion-reduce:transition-none";
