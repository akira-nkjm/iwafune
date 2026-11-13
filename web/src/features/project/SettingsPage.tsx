import { useDesign } from "@/api/queries";
import type { DesignView, SettingScope } from "@/api/types";
import { useT } from "@/i18n";
import { SkeletonPanel } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { TablePanel } from "@/kit/table/TablePanel";
import { LanguageSwitch } from "@/shell/LanguageSwitch";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";
import { DataPanel } from "./DataPanel";
import { SettingsTable } from "./SettingsTable";

/** 設定の区分（API の `SettingView.scope`）の並び。名前は i18n の project.groups */
const SCOPES: readonly SettingScope[] = ["conditions", "analysis", "budget"];

/** プロジェクト / 設定: プロジェクトの情報と、すべての設定（SetSetting）。CLI では fune setting set */
export function SettingsPage() {
  const p = useT().project;
  const design = useDesign();
  return (
    <>
      <PageHeader eyebrow={p.eyebrow} title={p.settings} />
      <PageBody>
        <QueryState
          query={design}
          className="flex flex-col gap-4"
          skeleton={
            <>
              <SkeletonPanel lines={3} />
              <SkeletonPanel lines={6} />
            </>
          }
        >
          {(d) => (
            <>
              <ProjectPanel design={d} />
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
        </QueryState>
      </PageBody>
    </>
  );
}

/** プロジェクトの情報（名前・場所・ブランチ・revision・スキーマの版）と言語の切り替え */
function ProjectPanel({ design: d }: { design: DesignView }) {
  const p = useT().project;
  const rows: [string, string][] = [
    [p.name, d.project.name],
    [p.root, d.project.root],
    [p.branch, d.project.branch ?? p.notGit],
    [p.revision, d.project.revision],
    [p.schema, String(d.project.schema)],
  ];
  return (
    <Panel>
      <PanelHeader title={p.projectPanel} right={<LanguageSwitch />} />
      <dl className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3 text-xs">
        {rows.map(([k, v]) => (
          <div key={k} className="min-w-0">
            <dt className="text-ink-3">{k}</dt>
            <dd className="truncate font-mono" title={v}>
              {v}
            </dd>
          </div>
        ))}
      </dl>
    </Panel>
  );
}
