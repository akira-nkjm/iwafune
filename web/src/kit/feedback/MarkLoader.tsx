import { brandHex } from "@/lib/tone";
import { MARK, MARK_BOX } from "@/shell/brand/brand-art";

/**
 * 待ちを示すロゴのマーク。3 つの面が順に明るくなる（周期は index.css の `mark-glow`。外形と稜線は動かさない）。
 * 図形は brand-art.ts の `MARK`、面の色は lib/tone.ts の `brandHex`。外形と稜線は文字色（`currentColor`）なので、
 * 明るい・暗いテーマのどちらの地でも読める。動きを減らす設定では面は動かず、すべて明るいまま。
 * 飾りなので画面読み上げには出さない（読み上げは `Working` / `Refreshing` の文が受け持つ）。
 */
export function MarkLoader({ size = 28, className }: { size?: number; className?: string }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${MARK_BOX} ${MARK_BOX}`}
      fill="none"
      aria-hidden="true"
      focusable="false"
      className={className}
    >
      <g transform={`translate(${MARK.shift.x} ${MARK.shift.y})`}>
        {MARK.faces.map((f, i) => (
          <path
            key={f.d}
            d={f.d}
            fill={brandHex[f.fill]}
            className="mark-glow"
            style={{ "--mark-i": i } as React.CSSProperties}
          />
        ))}
        <g
          stroke="currentColor"
          strokeWidth={MARK.strokeWidth}
          strokeLinejoin="round"
          strokeLinecap="round"
        >
          {[MARK.outline, ...MARK.ridges].map((d) => (
            <path key={d} d={d} />
          ))}
        </g>
      </g>
    </svg>
  );
}
