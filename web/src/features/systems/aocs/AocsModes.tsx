import { useQuery } from "@tanstack/react-query";
import { ChevronDown } from "lucide-react";
import { useId, useState } from "react";
import { q } from "@/api/queries";
import type { AocsModeView, DesignView, ModeHeader } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { SkeletonPanel } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { Reveal } from "@/kit/Reveal";
import { cn } from "@/lib/cn";
import { nodeName } from "@/lib/design";
import { quantityText } from "@/quantity/format";
import { AttitudeLine } from "./Attitude";
import { DeviceChips, Devices } from "./Devices";
import { ModeAnalysis, ModeValues } from "./ModeAnalysis";

/**
 * AOCS だけのモードカード。見比べられるように広い画面では横に並べ、編集と詳細はクリックで開く。
 * 集計と解析は system_modes の DTO をそのまま表示する。
 */
export function AocsModes({ design }: { design: DesignView }) {
  const query = useQuery(q.systemModes("aocs"));
  const t = useT();
  return (
    <QueryState query={query} skeleton={<SkeletonPanel lines={8} />}>
      {(view) => (
        <div className="space-y-3" aria-busy={query.isFetching}>
          <p className="text-ink-3 text-xs">{t.systems.aocsModes.basis}</p>
          {design.modes.modes.length === 0 && <Empty>{t.systems.aocsModes.empty}</Empty>}
          <div className="grid items-start gap-3 sm:grid-cols-2 xl:grid-cols-4">
            {view.modes.map((mode) => {
              const header = design.modes.modes.find((m) => m.id === mode.id);
              return header && mode.aocs ? (
                <AocsModeCard key={mode.id} header={header} item={mode.aocs} design={design} />
              ) : null;
            })}
          </div>
        </div>
      )}
    </QueryState>
  );
}

/** 1 モードの見比べる値（姿勢則・使う機器・3 つの値）。詳細は下のボタンで開く。 */
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
  const [open, setOpen] = useState(false);
  const detailsId = useId();
  const name = nodeName(header);
  return (
    <Panel className="flex flex-col gap-3">
      <PanelHeader
        className="mb-0"
        title={<span title={header.id}>{name}</span>}
        caption={`${t.maxDuration} ${quantityText(header.max_duration)}`}
      />
      <AttitudeLine item={item.attitude} mode={name} />
      <DeviceChips groups={item.devices} />
      <ModeValues item={item} />
      <div>
        <button
          type="button"
          aria-expanded={open}
          aria-controls={detailsId}
          onClick={() => setOpen((v) => !v)}
          className="-mx-1 flex w-[calc(100%+0.5rem)] items-center justify-between rounded-control px-1 py-0.5 text-ink-3 text-xs hover:bg-panel-2 hover:text-ink"
        >
          {open ? t.hideDetails : t.showDetails}
          <ChevronDown
            size={14}
            aria-hidden
            className={cn("transition-transform", open && "rotate-180")}
          />
        </button>
        <div id={detailsId}>
          <Reveal open={open}>
            <AocsModeDetails item={item} design={design} />
          </Reveal>
        </div>
      </div>
    </Panel>
  );
}

/** カードの詳細（機器の状態の内訳と蝕での変化、解析の状態と不足入力） */
export function AocsModeDetails({ item, design }: { item: AocsModeView; design: DesignView }) {
  return (
    <>
      <Devices groups={item.devices} eclipseGroups={item.eclipse_devices} defs={design.defs} />
      <ModeAnalysis item={item} />
    </>
  );
}
