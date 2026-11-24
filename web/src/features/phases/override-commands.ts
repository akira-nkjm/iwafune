import type { CommandText, PhaseFieldView, PhaseView } from "@/api/types";
import type { SettingRow } from "@/features/project/SettingsTable";

/** `fune phase copy-orbit <ID>`: 設計の軌道を上書きの軌道として丸ごと写す（上書きの出発点。1 手なので元に戻すのも 1 手） */
export function copyOrbitCommand(phase: string): CommandText {
  return { command: "phase_copy_orbit", phase };
}

/** `fune phase unset-orbit <ID>`: 軌道の上書きを丸ごと消す */
export function unsetOrbitCommand(phase: string): CommandText {
  return { command: "phase_unset_orbit", phase };
}

/** `fune phase unset-law <ID> <モード>`: そのモードの姿勢則の上書きを丸ごと消す */
export function unsetLawCommand(phase: string, mode: string): CommandText {
  return { command: "phase_unset_law", phase, mode };
}

/** 姿勢則の上書きの欄のパス（`law.<モード>.<キー>`。`fune phase set-field` のパスで、種類の欄は `law`） */
export function lawKindPath(mode: string): string {
  return `law.${mode}.law`;
}

/**
 * 上書きの欄を設定の表の行にする（API が返した欄のパス・値・表示をそのまま渡すだけ）。
 * 種類（`law`）を先に書けば、そのモードの上書きは読み直せるファイルになる（`AttitudeLawSetting` は種類が必須）。
 */
export function overrideRows(fields: readonly PhaseFieldView[], file: string): SettingRow[] {
  return fields.map((f) => ({
    key: f.path,
    field: f.key,
    value: f.value,
    display: f.display,
    file,
    choices: f.choices,
  }));
}

/** 姿勢則の上書きを足せるモード（設計のモードのうち、このフェーズがまだ上書きしていないもの。並びは設計のまま） */
export function addableModes(modeIds: readonly string[], phase: PhaseView): string[] {
  const taken = new Set(phase.laws.map((l) => l.mode));
  return modeIds.filter((id) => !taken.has(id));
}
