import { useState } from "react";
import type { AocsDeviceGroup, DefsView } from "@/api/types";
import { LightingSwitch } from "@/features/modes/matrix/ModesMatrix";
import { localized, useT } from "@/i18n";
import { RoleIcon } from "@/kit/RoleIcon";
import { ChipLink } from "@/kit/selection/Chip";
import { cn } from "@/lib/cn";
import { paths } from "@/lib/nav";

/** カードに出す役割のチップ（稼働台数 / 全台数）。1 台も使わない役割は点線で薄く出す。 */
export function DeviceChips({ groups }: { groups: AocsDeviceGroup[] }) {
  const t = useT().systems.aocsModes;
  return (
    <ul aria-label={t.devices} className="flex flex-wrap gap-1.5">
      {groups.map((group) => {
        const idle = group.active_count === 0;
        return (
          <li
            key={group.role ?? "other"}
            title={t.count(group.active_count, group.total_count)}
            data-idle={idle || undefined}
            className={cn(
              "inline-flex items-center gap-1 rounded-chip border px-1.5 py-0.5 text-[11px]",
              idle ? "border-dashed border-line-strong text-ink-4" : "border-line text-ink-2",
            )}
          >
            <RoleIcon role={group.role} size={12} />
            <span>{localized(group.label, group.label_en)}</span>
            <span className="num text-ink-3">
              {t.chipCount(group.active_count, group.total_count)}
            </span>
          </li>
        );
      })}
    </ul>
  );
}

/** API が役割別に集約した機器と状態。非稼働は薄く、不明は明示する。 */
export function Devices({
  groups,
  eclipseGroups,
  defs,
}: {
  groups: AocsDeviceGroup[];
  eclipseGroups: AocsDeviceGroup[];
  defs: DefsView;
}) {
  const [eclipse, setEclipse] = useState(false);
  const t = useT().systems.aocsModes;
  return (
    <section className="border-line border-t py-3">
      <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
        <h3 className="font-medium text-sm">{t.devices}</h3>
        <LightingSwitch value={eclipse} onChange={setEclipse} />
      </div>
      <div className="space-y-2">
        {(eclipse ? eclipseGroups : groups).map((group) => (
          <div key={group.role ?? "other"} className="text-xs">
            <div className="mb-1 flex justify-between gap-2 text-ink-3">
              <span>{localized(group.label, group.label_en)}</span>
              <span>{t.count(group.active_count, group.total_count)}</span>
            </div>
            <div className="flex flex-wrap gap-2">
              {group.devices.length === 0 && <span className="text-ink-4">—</span>}
              {group.devices.map((device) => (
                <div
                  key={device.part}
                  className={device.active === false ? "text-ink-4 opacity-50" : "text-ink-2"}
                >
                  <ChipLink to={paths.component(device.part)}>{device.part}</ChipLink>
                  <span className="ml-1">
                    {device.active === null
                      ? t.unknownState
                      : device.states
                          .map((state) => {
                            const label = defs.labels.state[state.state];
                            return `${label ? localized(label.label, label.label_en) : state.state} × ${state.count} (${state.fraction.display})`;
                          })
                          .join(" / ")}
                  </span>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
