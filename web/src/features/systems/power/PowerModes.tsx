import { useState } from "react";
import type { PowerModeView } from "@/api/types";
import { LightingSwitch } from "@/features/modes/matrix/ModesMatrix";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { SkeletonPanel } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Refreshable } from "@/kit/feedback/Refreshable";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { Segmented } from "@/kit/selection/Segmented";
import { PowerBudget } from "./PowerBudget";
import { powerModesOf, usePowerModes } from "./use-power-modes";

/** 最初に出すモードの数（残りは「すべて表示」） */
const SHORT_LIST = 6;
/** これより多いと行を細くする */
const THIN_ROWS = 10;

type Order = "tight" | "defined";

/**
 * 電源の「モード」タブ: 日照中（蝕中）の電力の行き先をモードごとの棒で比べる。
 * 既定は余裕の少ない順（API の `order` が小さい順）に最大 6 モード。定義の順にも切り替えられる
 */
export function PowerModes({ kase }: { kase: string | null }) {
  const query = usePowerModes(kase);
  return (
    <QueryState query={query} skeleton={<SkeletonPanel lines={6} />}>
      {(view) => (
        <Refreshable active={query.isFetching}>
          <PowerModesView modes={powerModesOf(view)} />
        </Refreshable>
      )}
    </QueryState>
  );
}

/** モードごとの棒（問い合わせは `PowerModes`）。 */
export function PowerModesView({ modes }: { modes: PowerModeView[] }) {
  const t = useT();
  const tp = t.systems.power;
  const [eclipse, setEclipse] = useState(false);
  const [order, setOrder] = useState<Order>("tight");
  const [all, setAll] = useState(false);
  const budgetOf = (m: PowerModeView) => (eclipse ? m.eclipse : m.sunlit);
  const withBudget = modes.filter((m) => budgetOf(m));
  const sorted =
    order === "tight"
      ? [...withBudget].sort((a, b) => (budgetOf(a)?.order ?? 0) - (budgetOf(b)?.order ?? 0))
      : withBudget;
  const shown = all ? sorted : sorted.slice(0, SHORT_LIST);
  return (
    <Panel>
      <PanelHeader
        title={eclipse ? tp.budgetEclipse : tp.budgetSunlit}
        caption={eclipse ? tp.budgetEclipseCaption : tp.budgetSunlitCaption}
        right={
          <div className="flex flex-wrap items-center gap-2">
            <Segmented<Order>
              size="sm"
              label={tp.order}
              value={order}
              onChange={setOrder}
              options={[
                { value: "tight", label: tp.orderTight },
                { value: "defined", label: tp.orderDefined },
              ]}
            />
            <LightingSwitch value={eclipse} onChange={setEclipse} />
          </div>
        }
      />
      {shown.length === 0 ? (
        <Empty>{tp.unavailable}</Empty>
      ) : (
        <>
          <PowerBudget modes={shown} eclipse={eclipse} thin={shown.length > THIN_ROWS} />
          {sorted.length > SHORT_LIST && (
            <button
              type="button"
              onClick={() => setAll((v) => !v)}
              className="mt-2 px-2 text-accent text-xs hover:underline"
            >
              {all ? tp.showFewer : tp.showAll(sorted.length)}
            </button>
          )}
        </>
      )}
    </Panel>
  );
}
