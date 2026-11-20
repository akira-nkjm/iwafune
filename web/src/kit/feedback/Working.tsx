import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { MarkLoader } from "./MarkLoader";
import { useAfter, useElapsedSeconds, WORKING_DELAY_MS } from "./use-working";

/** 経過秒の表示（1 秒ごとに変わる。画面読み上げには伝えない） */
function Elapsed() {
  const t = useT().common.working;
  return (
    <span aria-hidden="true" className="num text-ink-3">
      {t.elapsed(useElapsedSeconds())}
    </span>
  );
}

/** 画面読み上げに伝える文（「読み込み中…」と、何をしているか。秒は含めない） */
function SrText({ label }: { label: string }) {
  const t = useT();
  return (
    <span className="sr-only">
      {t.common.loading} {label}
    </span>
  );
}

/**
 * 表の行のように小さな場所で、前の中身を出したまま読み直しているときの小さなチップ（ロゴのマーク・していること・経過秒）。
 * 置かれて 1 秒たってから現れる。位置は `className` で決める。広い場所は `Refreshable`（中央のロゴ）を使う。
 */
export function WorkingChip({ label, className }: { label: string; className?: string }) {
  const shown = useAfter(WORKING_DELAY_MS);
  if (!shown) return null;
  return (
    <div
      role="status"
      aria-busy="true"
      className={cn(
        "loading-appear inline-flex items-center gap-1.5 rounded-chip border border-line bg-panel px-2 py-1 text-ink-2 text-xs shadow-sm",
        className,
      )}
    >
      <SrText label={label} />
      <MarkLoader size={16} className="shrink-0 text-ink-2" />
      <span aria-hidden="true">{label}</span>
      <Elapsed />
    </div>
  );
}

/**
 * 長い計算や描画の間、まだ出せる中身が無いときの表示（中央にロゴのマーク、していること 1 行、経過秒）。
 * `className` で置き場所を決める（スケルトンの上に重ねるなら `absolute inset-0`）。
 * `after` ミリ秒たってから現れる（既定は `WORKING_DELAY_MS`）。`announce={false}` は、すでに `Loading` の中にあって
 * 読み上げが重ならない（`Loading` が受け持つ）ときに使う。
 */
export function Working({
  label,
  className,
  after = WORKING_DELAY_MS,
  announce = true,
}: {
  label: string;
  className?: string;
  after?: number;
  announce?: boolean;
}) {
  const shown = useAfter(after);
  if (!shown) return null;
  return (
    <div
      {...(announce ? { role: "status", "aria-busy": true } : {})}
      className={cn(
        "loading-appear flex flex-col items-center justify-center gap-2 text-center text-ink-2 text-sm",
        className,
      )}
    >
      {announce && <SrText label={label} />}
      <MarkLoader size={44} className="text-ink-2" />
      <span aria-hidden={announce || undefined}>{label}</span>
      <Elapsed />
    </div>
  );
}
