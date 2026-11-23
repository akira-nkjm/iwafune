import { getT } from "@/i18n";
import type { ApiError } from "./client";
import { useCommandText, useSetText } from "./ops";

/**
 * 1 つの欄（設定・姿勢則の欄・軌道の欄）に打った文字列を書く口。欄の編集 UI は「値がどこから来て、どのコマンドで書くか」を
 * この形で受け取り、書き先（設計の設定か、フェーズの上書きか）を知らない。値の型を決めるのはどちらも service。
 */
export interface FieldEdit {
  /** 欄のパス（設計なら `conditions.orbit.altitude`、フェーズの上書きなら `orbit.altitude`・`law.NOMINAL.axis`）と打った文字列 */
  set: (path: string, text: string) => void;
  pending: boolean;
  error: ApiError | null;
}

/** 設計の設定（`conditions/`・`analysis/`・`budget/`）へ書く（`POST /ops/from_text` → `fune set` と同じ）。 */
export function useSettingEdit(): FieldEdit {
  const m = useSetText();
  return {
    set: (path, value) =>
      m.mutate({ path, value, summary: getT().quantity.setSummary(path, value) }),
    pending: m.isPending,
    error: m.error,
  };
}

/** フェーズの軌道・姿勢則の上書きへ書く（`POST /ops/from_command` の `phase_field` → `fune phase set-field` と同じ）。 */
export function usePhaseFieldEdit(phase: string): FieldEdit {
  const m = useCommandText();
  return {
    set: (path, value) =>
      m.mutate({
        command: { command: "phase_field", phase, path, value },
        summary: getT().phases.overridesDialog.setSummary(phase, path, value),
        paths: [path],
      }),
    pending: m.isPending,
    error: m.error,
  };
}
