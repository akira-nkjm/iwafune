import type { Connection, Edge } from "@xyflow/react";
import { useCallback, useState } from "react";
import { toast } from "sonner";
import { useApplyOps } from "@/api/ops";
import { getT } from "@/i18n";
import type { DiagramModel } from "../model";
import { connectIntent } from "./connect";

export type PendingNet = { a: { part: string; port: string }; b: { part: string; port: string } };

/** ハンドルどうしを繋ぐ（SetPort / 新しいネット）と、辺を消す（切り離し） */
export function useDiagramConnect(model: DiagramModel | null) {
  const { mutate } = useApplyOps();
  const [pending, setPending] = useState<PendingNet | null>(null);
  const onConnect = useCallback(
    (c: Connection) => {
      if (!model) return;
      const intent = connectIntent(model, c);
      if (intent.kind === "invalid") toast.error(intent.reason);
      else if (intent.kind === "new_net") setPending({ a: intent.a, b: intent.b });
      else
        mutate({
          ops: [{ op: "set_port", part: intent.part, port: intent.port, net: intent.net }],
          summary: getT().diagram.connectSummary(`${intent.part}.${intent.port}`, intent.net),
        });
    },
    [model, mutate],
  );
  const onEdgesDelete = useCallback(
    (removed: Edge[]) => {
      if (!model) return;
      const ops = removed.flatMap((r) => {
        const e = model.edges.find((x) => x.id === r.id);
        return e?.detach
          ? [{ op: "set_port" as const, part: e.detach.part, port: e.detach.port, net: null }]
          : [];
      });
      if (ops.length === 0) toast.message(getT().diagram.mergedDetach);
      else
        mutate({
          ops,
          summary: getT().diagram.detachSummary(ops.map((o) => `${o.part}.${o.port}`).join(", ")),
        });
    },
    [model, mutate],
  );
  return { onConnect, onEdgesDelete, pending, clearPending: () => setPending(null) };
}
