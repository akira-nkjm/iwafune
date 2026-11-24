import { buildModel, type DiagramModel } from "@/features/diagram/model";
import { diagramDemo } from "./diagram-model";
import { fixture } from "./fixture";

/** 配置の結果を比べるための入力（full が位置を決める表示モデル、model が線を引く表示モデル） */
export interface LayoutCase {
  name: string;
  full: DiagramModel;
  model: DiagramModel;
}

const layerOnly = (m: DiagramModel, layer: "power" | "signal"): DiagramModel => ({
  ...m,
  edges: m.edges.filter((e) => e.layer === layer),
});

/** 1 つの枠（とその中身、それに触れる線）を除いた表示モデル */
function without(m: DiagramModel, group: string): DiagramModel {
  const gone = new Set([group, ...m.cards.filter((c) => c.parent === group).map((c) => c.id)]);
  return {
    ...m,
    containers: m.containers.filter((c) => !gone.has(c.id)),
    cards: m.cards.filter((c) => !gone.has(c.id)),
    edges: m.edges.filter((e) => !gone.has(e.source.node) && !gone.has(e.target.node)),
  };
}

const small = buildModel(fixture, { layers: new Set(["power", "signal"]), mergeSiblings: true });
const noAocs = without(diagramDemo, "group:aocs");
const noPayload = without(diagramDemo, "group:payload");

/** 配置の同一性・キャッシュの鍵のテストで使う入力（デモとその変形、小さい設計） */
export const layoutCases: LayoutCase[] = [
  { name: "demo", full: diagramDemo, model: diagramDemo },
  { name: "demo-power", full: diagramDemo, model: layerOnly(diagramDemo, "power") },
  { name: "demo-signal", full: diagramDemo, model: layerOnly(diagramDemo, "signal") },
  { name: "demo-no-aocs", full: noAocs, model: noAocs },
  { name: "demo-no-payload", full: noPayload, model: noPayload },
  { name: "small", full: small, model: small },
];
