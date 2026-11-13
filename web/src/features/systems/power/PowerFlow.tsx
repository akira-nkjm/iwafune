import type { PowerFlowView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { systemTone } from "@/lib/systems";
import { toneHex } from "@/lib/tone";

/** API が作った流れの座標・電力・線幅を描く。 */
export function PowerFlow({ flow }: { flow: PowerFlowView }) {
  const t = useT().systems.power;
  return (
    <figure className="m-0 overflow-x-auto">
      <svg
        role="img"
        aria-label={t.flow}
        viewBox={`0 0 800 ${flow.height}`}
        className="min-w-[560px] w-full"
      >
        <title>{t.flow}</title>
        {flow.edges.map((edge) => (
          <g key={`${edge.x1}:${edge.y1}:${edge.x2}:${edge.y2}`}>
            <line
              x1={edge.x1}
              y1={edge.y1}
              x2={edge.x2}
              y2={edge.y2}
              stroke={toneHex[systemTone(edge.system)]}
              strokeWidth={edge.width}
              opacity={0.6}
            />
            <path
              d={`M ${edge.x2} ${edge.y2} l -8 -5 v 10 Z`}
              fill={toneHex[systemTone(edge.system)]}
            />
          </g>
        ))}
        {flow.nodes.map((node) => (
          <g key={node.id} transform={`translate(${node.x},${node.y})`}>
            <rect
              x={-73}
              y={-27}
              width={146}
              height={54}
              rx={9}
              fill="var(--color-panel)"
              stroke={toneHex[systemTone(node.system)]}
            />
            <text textAnchor="middle" y={-4} className="fill-ink text-[13px]">
              {localized(node.label.label, node.label.label_en)}
            </text>
            {node.value && (
              <text textAnchor="middle" y={16} className="num fill-ink-2 text-xs">
                {node.value.display}
              </text>
            )}
          </g>
        ))}
      </svg>
      <figcaption className="text-xs text-ink-3">{t.flowCaption}</figcaption>
    </figure>
  );
}
