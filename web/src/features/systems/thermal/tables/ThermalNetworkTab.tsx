import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useCommandOps } from "@/api/ops";
import { q } from "@/api/queries";
import { SkeletonPanel } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Reveal } from "@/kit/Reveal";
import { CoatingsPanel } from "./CoatingsPanel";
import { CouplingsPanel } from "./CouplingsPanel";
import { draftCommand, type ThermalDraft } from "./draft";
import { SurfacesPanel } from "./SurfacesPanel";
import { ThermalSaveBar } from "./ThermalSaveBar";

/**
 * 熱のページの「結合と表面」のタブ。結合・外面の表面材・表面材の 3 つの表を、保存していない下書きのまま直せる。
 * 下書きは変えたセル（`draft.ts`）で、service が `SetSetting` の列にし（`useCommandOps`）、書き込まずに当てた表
 * （`POST /thermal/tables`）を表に出す。保存済みの表との温度の差は保存の帯が前 → 後で見せる。
 */
export function ThermalNetworkTab({ kase }: { kase: string | null }) {
  const [draft, setDraft] = useState<ThermalDraft>(new Map());
  const made = useCommandOps(draftCommand(draft));
  const saved = useQuery(q.thermalTables(kase, []));
  const shown = useQuery(q.thermalTables(kase, made.latest ?? []));
  const view = shown.data ?? saved.data;
  return (
    <QueryState
      query={{ data: view, error: saved.error, refetch: saved.refetch }}
      skeleton={<SkeletonPanel lines={8} />}
    >
      {(v) => (
        <div className="space-y-4">
          <CouplingsPanel table={v.couplings} draft={draft} onDraft={setDraft} />
          <SurfacesPanel
            surfaces={v.surfaces}
            coatings={v.coatings}
            draft={draft}
            onDraft={setDraft}
          />
          <CoatingsPanel coatings={v.coatings} draft={draft} onDraft={setDraft} />
          <div className="sticky bottom-0 z-10 -mx-1">
            <Reveal open={draft.size > 0}>
              <ThermalSaveBar
                ops={made.ops}
                error={made.error}
                count={draft.size}
                before={saved.data?.temperatures ?? []}
                after={shown.data?.temperatures ?? []}
                working={shown.isFetching}
                onDone={() => setDraft(new Map())}
              />
            </Reveal>
          </div>
        </div>
      )}
    </QueryState>
  );
}
