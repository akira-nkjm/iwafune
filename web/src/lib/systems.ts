import {
  Cpu,
  Gauge,
  Layers3,
  type LucideIcon,
  Radio,
  Rocket,
  Sun,
  Telescope,
  Thermometer,
} from "lucide-react";
import type { System } from "@/api/types";
import type { Tone } from "./tone";

/**
 * 系 → 色とアイコンの唯一の対応（全系に必要なので、足し忘れは型の誤りになる）。表示名・ディレクトリ名・役割から色を推測しない。
 * 系ごとに一部の系だけが持つもの（専用のタブ・操作・追加の表示）は `features/systems/dedicated.tsx` の登録表。
 */
const looks: Record<System, { tone: Tone; icon: LucideIcon }> = {
  aocs: { tone: "blue", icon: Gauge },
  cdh: { tone: "navy", icon: Cpu },
  comm: { tone: "gold", icon: Radio },
  payload: { tone: "rose", icon: Telescope },
  power: { tone: "green", icon: Sun },
  propulsion: { tone: "magenta", icon: Rocket },
  structure: { tone: "violet", icon: Layers3 },
  thermal: { tone: "orange", icon: Thermometer },
};

/** API が返す系の色。未所属は中立色。 */
export function systemTone(system: System | null | undefined): Tone {
  return system ? looks[system].tone : "gray";
}

/** 系のアイコン（ナビの項目） */
export function systemIcon(system: System): LucideIcon {
  return looks[system].icon;
}

/** 文字列が系の ID か（登録表の対応で型を確かめる） */
export function isSystem(key: string): key is System {
  return Object.hasOwn(looks, key);
}
