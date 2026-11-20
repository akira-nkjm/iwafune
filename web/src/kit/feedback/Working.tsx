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
