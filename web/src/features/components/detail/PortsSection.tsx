import { useApplyOps } from "@/api/ops";
import type { DesignView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { PropertyList } from "@/kit/detail/PropertyList";
import { PropertyRow } from "@/kit/detail/PropertyRow";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { Pill } from "@/kit/status/Pill";
import { type PortRole, portRole, withNote } from "@/lib/design";

const roleTone = {
  power_in: "pos",
  power_out: "pos",
  data: "accent",
  rf: "warn",
  none: "neutral",
} as const;

/** 接続: ポートごとの所属ネット（SetPort）。ポートの種類は所属するネットの種類から決まる */
export function PortsSection({ design, part }: { design: DesignView; part: PartRow }) {
  const apply = useApplyOps();
  const t = useT().components.ports;
  const roleLabel: Record<PortRole, string> = {
    power_in: t.powerIn,
    power_out: t.powerOut,
    data: t.data,
    rf: t.rf,
    none: t.unconnected,
  };
  if (part.ports.length === 0) return <p className="px-1 text-ink-3 text-xs">{t.none}</p>;
  return (
    <PropertyList>
      {part.ports.map((p) => {
        const role = portRole(design, part.id, p.id, p.net);
        return (
          <PropertyRow
            key={p.id}
            label={
              <span className="flex items-center gap-2">
                <span className="font-mono text-ink">{p.id}</span>
                <Pill tone={roleTone[role]} dot={false}>
                  {roleLabel[role]}
                </Pill>
              </span>
            }
          >
            <NativeSelect
              size="sm"
              aria-label={t.netOf(`${part.id}.${p.id}`)}
              value={p.net ?? ""}
              placeholder={t.unconnectedOption}
              onChange={(e) =>
                apply.mutate({
                  ops: [{ op: "set_port", part: part.id, port: p.id, net: e.target.value || null }],
                  summary: e.target.value
                    ? t.connectSummary(`${part.id}.${p.id}`, e.target.value)
                    : t.detachSummary(`${part.id}.${p.id}`),
                })
              }
              options={design.nets.map((n) => ({
                value: n.id,
                label: withNote(
                  n.id,
                  [n.kind, n.rail?.display ?? n.protocol].filter(Boolean).join(" "),
                ),
              }))}
            />
          </PropertyRow>
        );
      })}
    </PropertyList>
  );
}
