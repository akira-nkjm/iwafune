import { useQuery } from "@tanstack/react-query";
import { useFetchData } from "@/api/data";
import { q } from "@/api/queries";
import type { DataItem } from "@/api/types";
import { useT } from "@/i18n";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { Button } from "@/ui/button";

/** 外部データ（天体の暦・宇宙天気の表）の状態と取得。状態の検証はサービスが行い、ここは表示するだけ */
export function DataPanel() {
  const words = useT().project.data;
  const status = useQuery(q.dataStatus());
  const fetchData = useFetchData();
  const error = status.error ?? fetchData.error;
  return (
    <Panel>
      <PanelHeader
        title={words.title}
        caption={status.data ? words.ephemeris[status.data.ephemeris] : undefined}
      />
      <ul className="space-y-2">
        {status.data?.items.map((item) => (
          <li key={item.id} className="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-xs">
            <span>
              <span className="font-mono">{item.id}</span>: {stateText(item, words)}{" "}
              {item.size?.display}
            </span>
            {item.fetched_at && (
              <span className="font-mono text-ink-3">{words.fetchedAt(item.fetched_at)}</span>
            )}
            <Button
              variant="outline"
              size="xs"
              disabled={fetchData.isPending}
              onClick={() => fetchData.mutate(item.id)}
            >
              {fetchData.isPending && fetchData.variables === item.id
                ? words.fetching
                : words.fetch}
            </Button>
            {item.install_error && (
              <span role="alert" className="text-neg-ink">
                {item.install_error}
              </span>
            )}
            <span className="basis-full text-ink-3">
              {item.used_by}
              {item.license && ` — ${item.license}`}
            </span>
          </li>
        ))}
      </ul>
      {error && (
        <p role="alert" className="mt-2 text-neg-ink text-xs">
          {error.message}
        </p>
      )}
    </Panel>
  );
}

/** 1 つのデータの状態（読み込み済み・キャッシュ済み・未取得） */
function stateText(
  item: DataItem,
  words: { installed: string; cached: string; missing: string },
): string {
  if (item.installed) return words.installed;
  return item.present ? words.cached : words.missing;
}
