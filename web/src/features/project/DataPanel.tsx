import { useMutation, useQuery } from "@tanstack/react-query";
import { api, seg } from "@/api/client";
import type { DataStatus } from "@/api/gen";
import { useT } from "@/i18n";
import { Panel } from "@/kit/Panel";
import { PanelHeader } from "@/kit/PanelHeader";

/** 外部データの状態と取得。サービスの検証結果を表示する。 */
export function DataPanel() {
  const t = useT();
  const words = t.project.data;
  const status = useQuery({
    queryKey: ["data-status"],
    queryFn: () => api<DataStatus>("/data/status"),
  });
  const { refetch: refresh } = status;
  const fetchData = useMutation({
    mutationFn: (id: string) => api<DataStatus>(`/data/${seg(id)}/fetch`, { method: "POST" }),
    onSuccess: () => {
      void refresh();
    },
  });
  return (
    <Panel>
      <PanelHeader title="DE440" caption={status.data?.ephemeris} />
      {status.data?.items.map((item) => (
        <div key={item.id} className="flex items-center gap-3 text-xs">
          <span>
            {item.id}:{" "}
            {item.installed ? words.installed : item.present ? words.cached : words.missing}{" "}
            {item.size_display?.display}
          </span>
          <button
            type="button"
            disabled={fetchData.isPending}
            onClick={() => fetchData.mutate(item.id)}
          >
            {fetchData.isPending ? words.fetching : words.fetch}
          </button>
          {item.install_error && <span role="alert">{item.install_error}</span>}
        </div>
      ))}
      {(status.error || fetchData.error) && (
        <p role="alert">{(status.error || fetchData.error)?.message}</p>
      )}
    </Panel>
  );
}
