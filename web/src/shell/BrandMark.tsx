/**
 * ロゴのマーク（天磐船の船体）。図形は brand-art.ts の `MARK`、色は lib/tone.ts の `brandHex` で、
 * テーマには追従させない。
 */

import { brandHex, brandInkHex } from "@/lib/tone";
import { MARK, MARK_BOX } from "./brand-art";

const SQUARE = { x: 0, y: 0, w: MARK_BOX, h: MARK_BOX } as const;

/** `size` は幅 [px]。`tight` のときは余白を切り詰め（`MARK.tight`）、高さは図形の縦横比で決まる */
export function BrandMark({ size = 18, tight = false }: { size?: number; tight?: boolean }) {
  const box = tight ? MARK.tight : SQUARE;
  return (
    <svg
      width={size}
      height={(size * box.h) / box.w}
      viewBox={`${box.x} ${box.y} ${box.w} ${box.h}`}
      fill="none"
      aria-hidden="true"
      focusable="false"
    >
      <g transform={`translate(${MARK.shift.x} ${MARK.shift.y})`}>
        {MARK.faces.map((f) => (
          <path key={f.d} d={f.d} fill={brandHex[f.fill]} />
        ))}
        <g
          stroke={brandInkHex.light.outline}
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
