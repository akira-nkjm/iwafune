import { useQuery } from "@tanstack/react-query";
import { q } from "@/api/queries";
import type { AocsModeView, DesignView, ModeHeader } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { SkeletonPanel } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { Pill } from "@/kit/status/Pill";
import { nodeName } from "@/lib/design";
import { quantityText } from "@/quantity/format";
import { Attitude } from "./Attitude";
import { Devices } from "./Devices";
import { ModeAnalysis } from "./ModeAnalysis";

/** AOCS だけのモードカード。集計と解析は system_modes の DTO をそのまま表示する。 */
export function AocsModes({ design }: { design: DesignView }) {
  const query = useQuery(q.systemModes("aocs"));
  const t = useT();
  return (
    <QueryState query={query} skeleton={<SkeletonPanel lines={8} />}>
      {(view) => (
        <div className="grid gap-4 xl:grid-cols-2" aria-busy={query.isFetching}>
          {design.modes.modes.length === 0 && <Empty>{t.systems.aocsModes.empty}</Empty>}
          {view.modes.map((mode) => {
            const header = design.modes.modes.find((m) => m.id === mode.id);
            return header && mode.aocs ? (
              <AocsModeCard key={mode.id} header={header} item={mode.aocs} design={design} />
            ) : null;
          })}
        </div>
      )}
    </QueryState>
  );
}

/** 1 モードの姿勢・機器・解析値。 */
export function AocsModeCard({
  header,
  item,
  design,
}: {
  header: ModeHeader;
  item: AocsModeView;
  design: DesignView;
}) {
  const t = useT().systems.aocsModes;
  return (
    <Panel>
      <PanelHeader
        title={nodeName(header)}
        caption={header.id}
        right={
          <Pill dot={false}>
            {t.maxDuration}: {quantityText(header.max_duration)}
          </Pill>
        }
      />
      <Attitude item={item.attitude} />
      <Devices groups={item.devices} eclipseGroups={item.eclipse_devices} defs={design.defs} />
      <ModeAnalysis item={item} />
      <p className="mt-3 text-xs text-ink-3">{t.basis}</p>
    </Panel>
  );
}
