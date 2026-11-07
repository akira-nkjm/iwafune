import type { DesignView, NetView, PartRow } from "@/api/types";
import { nodeName } from "@/lib/design";
import { subsystemTone } from "@/lib/subsystems";
import { type Tone, toneHex, uiHex } from "@/lib/tone";
import { type Layer, layerOf, type PowerCategory, powerCategories } from "./power";

/**
 * 系統図の表示モデル。設計（部品・製品ツリー・ネット）から、サブシステムの枠・部品のカード・辺を組み立てる。
 * 位置は持たない（layout.ts が決める）。値の計算はせず、つながりを並べ替えるだけ。
 */
export interface DiagramOptions {
  layers: ReadonlySet<Layer>;
  /** 同じカタログ・同じ接続の兄弟の部品を 1 枚の「×N」のカードにまとめる */
  mergeSiblings: boolean;
  /**
   * 枠（サブシステム）の名前の言語。`"en"` は ID（AOCS など）だけ、`"ja"` は設計に書いた表示名だけ（無ければ ID）。
   * 2 つの言語を並べると見出しが長くなり、読みにくいので 1 つにする。省くと `"ja"`。
   */
  lang?: "ja" | "en";
}

export interface DiagramContainer {
  id: string;
  label: string;
  tone: Tone;
  parent: string | null;
}

export interface DiagramCard {
  id: string;
  label: string;
  role?: string | null;
  sub: string;
  tone: Tone;
  /** カードに含まれる部品（まとめたときは複数） */
  members: string[];
  /** 員数 × まとめた数（1 なら重ねない） */
  count: number;
  /** 親: 組立品のカード（`card:` で始まる）か、サブシステムの枠（`group:`）か、なし */
  parent: string | null;
  ports: { id: string; port: string }[];
  assembly: boolean;
}

export interface DiagramJunction {
  id: string;
  net: string;
  parent: string | null;
}

export interface DiagramEdge {
  id: string;
  layer: Layer;
  nets: string[];
  /** 両端の端子の名前（`PCDU.OUT2>CAM.PWR`。ジャンクションは `(ネット)`）。線を指すキーに使う（routes.ts） */
  ends: string;
  source: { node: string; port: string | null };
  target: { node: string; port: string | null };
  label: string | null;
  sourceLabel: string | null;
  targetLabel: string | null;
  color: string;
  dashed: boolean;
  /** 消したときに切り離すポート（まとめたカードでは null） */
  detach: { part: string; port: string } | null;
}

export interface DiagramModel {
  containers: DiagramContainer[];
  cards: DiagramCard[];
  junctions: DiagramJunction[];
  edges: DiagramEdge[];
  legend: PowerCategory[];
}

export const groupNodeId = (id: string) => `group:${id}`;
export const cardNodeId = (id: string) => `card:${id}`;
export const portHandleId = (port: string) => port;

const SIGNAL_COLOR = toneHex.blue;
const RF_COLOR = toneHex.rose;

function commonPrefix(ids: string[]): string {
  let p = ids[0] ?? "";
  for (const id of ids) while (!id.startsWith(p)) p = p.slice(0, -1);
  return p.replace(/[_-]+$/, "");
}

function signature(p: PartRow): string {
  const ports = p.ports.map((x) => `${x.id}=${x.net ?? ""}`).sort();
  return `${p.catalog ?? ""}|${p.kind}|${ports.join(",")}`;
}

interface CardIndex {
  cardOfPart: Map<string, string>;
  cards: DiagramCard[];
}

/** 部品 → カード（組立品は子を中に持つカード、同種の兄弟は 1 枚にまとめる） */
function buildCards(design: DesignView, merge: boolean): CardIndex {
  const tree = new Map(design.tree.map((n) => [n.id, n]));
  const parts = new Map(design.parts.map((p) => [p.id, p]));
  const isAssembly = (id: string) => tree.get(id)?.children.some((c) => parts.has(c)) ?? false;
  const parentNode = (p: PartRow): string | null => {
    if (!p.parent) return null;
    return parts.has(p.parent) ? cardNodeId(p.parent) : groupNodeId(p.parent);
  };
  const buckets = new Map<string, PartRow[]>();
  for (const p of design.parts.toSorted((a, b) => a.id.localeCompare(b.id))) {
    const key =
      merge && p.catalog && !isAssembly(p.id) ? `${p.parent ?? ""}#${signature(p)}` : `one#${p.id}`;
    buckets.set(key, [...(buckets.get(key) ?? []), p]);
  }
  const cardOfPart = new Map<string, string>();
  const cards: DiagramCard[] = [];
  for (const group of buckets.values()) {
    const first = group[0];
    if (!first) continue;
    const merged = group.length > 1;
    const id = cardNodeId(first.id);
    for (const p of group) cardOfPart.set(p.id, id);
    const count = group.reduce((n, p) => n + p.count, 0);
    const prefix = merged ? commonPrefix(group.map((p) => p.id)) : first.id;
    cards.push({
      id,
      label: merged ? prefix || first.kind : first.id,
      sub: first.catalog ?? first.kind,
      role: first.role,
      tone: subsystemTone(first.subsystem, first.role),
      members: group.map((p) => p.id),
      count,
      parent: parentNode(first),
      ports: first.ports.map((x) => ({ id: `${id}::${x.id}`, port: x.id })),
      assembly: isAssembly(first.id),
    });
  }
  return { cardOfPart, cards };
}

function containersOf(design: DesignView, lang: "ja" | "en"): DiagramContainer[] {
  return design.tree
    .filter((n) => n.node === "group")
    .map((n) => ({
      id: groupNodeId(n.id),
      label: nodeName(n, lang),
      tone: subsystemTone(n.id),
      parent: n.parent ? groupNodeId(n.parent) : null,
    }));
}

type End = { card: string; port: string; part: string };

/**
 * 星形の中心と線の向き。電源は供給元、データはマスターが中心で、線は中心から各端へ向ける。
 * どちらも書かれていなければ、電源は PCDU・データは OBC を中心にする（無ければ null）。
 * 供給元の書かれていない電源のネット（太陽電池の母線など）は、電力が中心の配電へ集まるので、
 * 線は各端から中心へ向ける（`collects`）。
 */
function hubOf(
  design: DesignView,
  net: NetView,
  ends: End[],
): { hub: End | null; collects: boolean } {
  const ref = net.kind === "power" ? net.source : net.master;
  const byRef = ends.find((e) => e.part === ref?.part && e.port === ref.port);
  if (byRef) return { hub: byRef, collects: false };
  const role = net.kind === "power" ? "pcdu" : "obc";
  const hub = ends.find((e) => design.parts.find((p) => p.id === e.part)?.role === role) ?? null;
  return { hub, collects: net.kind === "power" && hub !== null };
}

/** 線の端（カードのポートかジャンクション）、端に添えるポート名、キーに使う名前（`部品.ポート`） */
type Endpoint = { node: string; port: string | null; label: string | null; name: string };
const endpointOf = (e: End): Endpoint => ({
  node: e.card,
  port: `${e.card}::${e.port}`,
  label: e.port,
  name: `${e.part}.${e.port}`,
});

export function buildModel(design: DesignView, opts: DiagramOptions): DiagramModel {
  const { cardOfPart, cards } = buildCards(design, opts.mergeSiblings);
  const containers = containersOf(design, opts.lang ?? "ja");
  const categories = powerCategories(design);
  const cardById = new Map(cards.map((c) => [c.id, c]));
  const junctions: DiagramJunction[] = [];
  const raw: DiagramEdge[] = [];
  const usedPorts = new Set<string>();

  for (const net of design.nets.toSorted((a, b) => a.id.localeCompare(b.id))) {
    const layer = layerOf(net);
    if (!opts.layers.has(layer)) continue;
    const seen = new Set<string>();
    const ends: End[] = [];
    for (const m of net.members) {
      const { part, port } = m;
      const card = part ? cardOfPart.get(part) : undefined;
      if (!part || !port || !card || seen.has(`${card}::${port}`)) continue;
      seen.add(`${card}::${port}`);
      ends.push({ card, port, part });
    }
    if (ends.length < 2) continue;
    const cat = categories.get(net.id);
    const color =
      layer === "signal"
        ? net.kind === "rf"
          ? RF_COLOR
          : SIGNAL_COLOR
        : (cat?.color ?? uiHex.ink3);
    const mergedEnd = (e: End) => (cardById.get(e.card)?.members.length ?? 1) > 1;
    // member: 中心でない側の端（線を消すとこのポートを切り離す）
    const edge = (from: Endpoint, to: Endpoint, member: End, i: number): DiagramEdge => ({
      id: `${net.id}#${i}`,
      layer,
      nets: [net.id],
      ends: `${from.name}>${to.name}`,
      source: { node: from.node, port: from.port },
      target: { node: to.node, port: to.port },
      label: layer === "signal" ? (net.protocol ?? net.id) : null,
      sourceLabel: from.label,
      targetLabel: to.label,
      color,
      dashed: net.kind === "rf",
      detach: mergedEnd(member) ? null : { part: member.part, port: member.port },
    });
    const { hub, collects } = hubOf(design, net, ends);
    if (hub || ends.length === 2) {
      const h = hub ?? (ends[0] as End);
      ends
        .filter((e) => e !== h)
        .forEach((e, i) => {
          const [from, to] = collects ? [e, h] : [h, e];
          raw.push(edge(endpointOf(from), endpointOf(to), e, i));
        });
    } else {
      const j = `junction:${net.id}`;
      const parents = new Set(ends.map((e) => cardById.get(e.card)?.parent ?? null));
      junctions.push({
        id: j,
        net: net.id,
        parent: parents.size === 1 ? ([...parents][0] ?? null) : null,
      });
      ends.forEach((e, i) => {
        raw.push(
          edge({ node: j, port: null, label: null, name: `(${net.id})` }, endpointOf(e), e, i),
        );
      });
    }
  }

  const edges = aggregate(raw);
  for (const e of edges) {
    if (e.source.port) usedPorts.add(e.source.port);
    if (e.target.port) usedPorts.add(e.target.port);
  }
  const legend = [
    ...new Map(
      [...categories.entries()]
        .filter(([n]) => {
          const net = design.nets.find((x) => x.id === n);
          return net ? opts.layers.has(layerOf(net)) : false;
        })
        .map(([, c]) => [c.key, c]),
    ).values(),
  ].sort((a, b) => a.order - b.order);

  return {
    containers,
    cards: cards.map((c) => ({
      ...c,
      ports: c.ports.filter((p) => usedPorts.has(p.id) || p.port === ""),
    })),
    junctions,
    edges,
    legend,
  };
}

/** 同じ 2 枚のカードの間の、同じプロトコル（電源は同じ系統）の辺を 1 本にまとめて「×N」にする */
function aggregate(raw: DiagramEdge[]): DiagramEdge[] {
  const by = new Map<string, DiagramEdge[]>();
  for (const e of raw) {
    const key = `${e.source.node}|${e.target.node}|${e.layer}|${e.layer === "signal" ? e.label : e.color}`;
    by.set(key, [...(by.get(key) ?? []), e]);
  }
  return [...by.values()].map((list) => {
    const first = list[0] as DiagramEdge;
    if (list.length === 1) return first;
    const uniq = (xs: (string | null)[]) =>
      [...new Set(xs.filter((x): x is string => !!x))].join(",");
    return {
      ...first,
      id: list.map((e) => e.id).join("+"),
      nets: [...new Set(list.flatMap((e) => e.nets))],
      label: first.layer === "signal" ? `${first.label ?? ""}×${list.length}` : `×${list.length}`,
      sourceLabel: uniq(list.map((e) => e.sourceLabel)),
      targetLabel: uniq(list.map((e) => e.targetLabel)),
      detach: null,
    };
  });
}
