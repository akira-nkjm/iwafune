import type { Connection } from "@xyflow/react";
import { getT } from "@/i18n";
import type { DiagramModel } from "../model";

/** ドラッグで 2 つのハンドルを繋いだときの意味 */
export type ConnectIntent =
  | { kind: "join"; part: string; port: string; net: string }
  | { kind: "new_net"; a: { part: string; port: string }; b: { part: string; port: string } }
  | { kind: "invalid"; reason: string };

type End = { part: string; port: string; net: string | null } | { junction: string };

function endOf(
  model: DiagramModel,
  node: string,
  handle: string | null | undefined,
  nets: Map<string, string | null>,
): End | null {
  if (node.startsWith("junction:")) return { junction: node.slice("junction:".length) };
  const card = model.cards.find((c) => c.id === node);
  const port = card?.ports.find((p) => p.id === handle);
  const part = card?.members[0];
  if (!card || !port || !part || card.members.length > 1) return null;
  return { part, port: port.port, net: nets.get(`${part}.${port.port}`) ?? null };
}

/**
 * 繋いだ 2 点から Op を決める: 分岐点に落とす → そのネットに参加 / 片方がネット付き → そちらに参加 /
 * 両方未接続 → 新しいネット。まとめたカード（×N）のポートは繋げない（どの部品か決まらないため）。
 */
export function connectIntent(
  model: DiagramModel,
  c: Pick<Connection, "source" | "target" | "sourceHandle" | "targetHandle">,
  portNets: Map<string, string | null> = new Map(),
): ConnectIntent {
  const t = getT().diagram.connect;
  const a = endOf(model, c.source, c.sourceHandle, portNets);
  const b = endOf(model, c.target, c.targetHandle, portNets);
  if (!a || !b)
    return {
      kind: "invalid",
      reason: t.merged,
    };
  if ("junction" in a && "junction" in b) return { kind: "invalid", reason: t.junctions };
  if ("junction" in a && !("junction" in b))
    return { kind: "join", part: b.part, port: b.port, net: a.junction };
  if ("junction" in b && !("junction" in a))
    return { kind: "join", part: a.part, port: a.port, net: b.junction };
  if ("junction" in a || "junction" in b) return { kind: "invalid", reason: t.cannot };
  if (a.part === b.part) return { kind: "invalid", reason: t.samePart };
  if (a.net && b.net)
    return a.net === b.net
      ? { kind: "invalid", reason: t.sameNet }
      : {
          kind: "invalid",
          reason: t.otherNets(a.net, b.net),
        };
  if (a.net) return { kind: "join", part: b.part, port: b.port, net: a.net };
  if (b.net) return { kind: "join", part: a.part, port: a.port, net: b.net };
  return { kind: "new_net", a: { part: a.part, port: a.port }, b: { part: b.part, port: b.port } };
}
