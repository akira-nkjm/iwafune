import { Link } from "react-router";
import type { DefsView, PowerPageView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { TablePanel } from "@/kit/table/TablePanel";
import { paths } from "@/lib/nav";
import { PowerPartValues } from "./PowerPartValues";

/** 役割・列の定義は API。負荷は状態ごとの電力を表示する。 */
export function PowerParts({ page, defs }: { page: PowerPageView; defs: DefsView }) {
  const t = useT();
  return (
    <>
      {page.parts.map((group) => (
        <TablePanel key={group.key} title={localized(group.label.label, group.label.label_en)}>
          {group.parts.length === 0 ? (
            <Empty>{t.systems.noParts}</Empty>
          ) : (
            <div className="space-y-4">
              {group.parts.map((part) => {
                const id = part.id;
                return (
                  <div key={id} className="rounded-control border border-line p-3">
                    <Link to={paths.component(id)} className="font-mono text-sm hover:underline">
                      {id}
                    </Link>
                    <PowerPartValues part={part} attrs={group.attributes} defs={defs} />
                  </div>
                );
              })}
            </div>
          )}
        </TablePanel>
      ))}
    </>
  );
}
