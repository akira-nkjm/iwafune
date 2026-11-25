import type { CommandText, DesignView } from "@/api/types";
import type { SelectOption } from "@/kit/forms/NativeSelect";
import { idWithName } from "@/lib/design";

/** 解析ケースを足す欄（打った文字列のまま。量・周回数・時刻・`side` の読み方は service の `case_set_op`） */
export interface NewCase {
  id: string;
  label: string;
  description: string;
  mode: string;
  phase: string;
  side: string;
  thermal_case: string;
  beta: string;
  orbits: string;
  sun_distance: string;
  epoch: string;
}

/** 空の欄（足りない入力がケースの ID を名指ししていれば、`initialId` で始める） */
export function emptyCase(initialId?: string | null): NewCase {
  return {
    id: initialId ?? "",
    label: "",
    description: "",
    mode: "",
    phase: "",
    side: "",
    thermal_case: "",
    beta: "",
    orbits: "",
    sun_distance: "",
    epoch: "",
  };
}

/**
 * `fune case set` と同じ引数（`POST /ops/from_command` の `case_set`）。ID が空の間は作らない。
 * ほかの欄は空でよい（書かなかった欄は書かれない）。
 */
export function caseSetCommand(c: NewCase): CommandText | null {
  if (!c.id.trim()) return null;
  return { command: "case_set", ...c };
}

/** 設計のモード・フェーズ・熱の環境条件から選ぶ選択肢（すべて API が返した一覧。手で書いた一覧は持たない） */
export function caseChoices(design: DesignView): {
  modes: SelectOption[];
  phases: SelectOption[];
  thermalCases: SelectOption[];
} {
  return {
    modes: design.modes.modes.map((m) => ({ value: m.id, label: idWithName(m) })),
    phases: design.phases.map((p) => ({ value: p.id, label: idWithName(p) })),
    thermalCases: design.add_choices.thermal_cases.map((c) => ({ value: c, label: c })),
  };
}
