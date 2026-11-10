import { useDesign } from "@/api/queries";
import type { SettingScope } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/Empty";
import { Loading, SkeletonPanel } from "@/kit/Loading";
import { Panel } from "@/kit/Panel";
import { PanelHeader } from "@/kit/PanelHeader";
import { TablePanel } from "@/kit/TablePanel";
import { LanguageSwitch } from "@/shell/LanguageSwitch";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";
import { DataPanel } from "./DataPanel";
import { SettingsTable } from "./SettingsTable";

/** 設定の区分（API の `SettingView.scope`）の並び。名前は i18n の project.groups */
const SCOPES: readonly SettingScope[] = ["mission", "analysis", "budget"];

/** プロジェクト / 設定: プロジェクトの情報と、すべての設定（SetSetting）。CLI では fune setting set */
export function SettingsPage() {
  const t = useT();
  const p = t.project;
  const design = useDesign();
  const d = design.data;
  return (
    <>
      <PageHeader eyebrow={p.eyebrow} title={p.settings} />
      <PageBody>
        {!d ? (
          design.isError ? (
            <Empty>{t.common.cannotLoadDesign(design.error.message)}</Empty>
          ) : (
            <Loading className="space-y-4">
              <SkeletonPanel lines={3} />
              <SkeletonPanel lines={6} />
            </Loading>
          )
        ) : (
          <>
            <Panel>
              <PanelHeader title={p.projectPanel} right={<LanguageSwitch />} />
              <dl className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3 text-xs">
                {[
                  [p.name, d.project.name],
                  [p.root, d.project.root],
                  [p.branch, d.project.branch ?? p.notGit],
                  ["revision", d.project.revision],
                  ["schema", String(d.project.schema)],
                ].map(([k, v]) => (
                  <div key={k} className="min-w-0">
                    <dt className="text-ink-3">{k}</dt>
                    <dd className="truncate font-mono" title={v}>
                      {v}
                    </dd>
                  </div>
                ))}
              </dl>
            </Panel>
            <DataPanel />
            {SCOPES.map((scope) => (
              <TablePanel
                key={scope}
                title={p.groups[scope].title}
                caption={p.groups[scope].caption}
              >
                <SettingsTable settings={d.settings.filter((s) => s.scope === scope)} />
              </TablePanel>
            ))}
          </>
        )}
      </PageBody>
    </>
  );
}
