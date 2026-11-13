import { useState } from "react";
import type { AocsDeviceGroup, DefsView } from "@/api/types";
import { LightingSwitch } from "@/features/modes/matrix/ModesMatrix";
import { localized, useT } from "@/i18n";
import { ChipLink } from "@/kit/selection/Chip";
import { paths } from "@/lib/nav";

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
          <div key={group.role} className="text-xs">
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
