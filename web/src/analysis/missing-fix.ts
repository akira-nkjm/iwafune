import type { CommandText, DesignView, Fix, ListKind, MissingInput, SetText } from "@/api/types";
import { paths } from "@/lib/nav";

/** 値を入れる候補（その場のダイアログ） */
export type ValueFix = Extract<Fix, { kind: "set_value" }>;
/** 部品を選ぶ候補（その場のダイアログ） */
export type PickFix = Extract<Fix, { kind: "pick_part" }>;

/** 候補が開く画面（その場のダイアログを持たない候補は、既存のページへ行く。候補の種類で決め、文字列は読まない） */
export type RowAction =
  | { kind: "value"; fix: ValueFix }
  | { kind: "pick"; fix: PickFix }
  | {
      kind: "link";
      label: "setOnPart" | "addPart" | "assignPart" | { addItem: ListKind };
      to: string;
    };

/** 一覧への追加の画面（一覧の種類 → 既存のページ） */
function listPage(list: ListKind): string {
  switch (list) {
    case "mode":
      return paths.modes();
    case "maneuver":
      return paths.maneuvers();
    case "ground_station":
      return paths.orbit();
    case "comm_link":
      return paths.settings();
    case "case":
      return paths.cases();
  }
}

/** 候補 1 つの操作（その場の編集か、既存の画面への道） */
function actionOf(fix: Fix): RowAction {
  switch (fix.kind) {
    case "set_value":
      return { kind: "value", fix };
    case "pick_part":
      return { kind: "pick", fix };
    case "set_value_on_part":
      return { kind: "link", label: "setOnPart", to: paths.components() };
    case "add_part":
      return { kind: "link", label: "addPart", to: paths.addComponent() };
    case "assign_part":
      return { kind: "link", label: "assignPart", to: paths.components() };
    case "add_list_item":
      return { kind: "link", label: { addItem: fix.list }, to: listPage(fix.list) };
  }
}

/** 不足入力の行に出す操作（API が返した候補を、その場の編集か既存の画面への道に写す。候補の無い行は空） */
export function rowActions(m: MissingInput): RowAction[] {
  return m.fixes.map(actionOf);
}

/** 値の書き込み先（部品の上書きか、部品が選んだカタログ品か） */
export type ValueScope = "part" | "catalog";

/** 打った値 → `POST /ops/from_text` の入力（書き先の組は候補が持つ。値の型を決めるのは service）。空なら null */
export function valueSetText(fix: ValueFix, scope: ValueScope, value: string): SetText | null {
  const text = value.trim();
  if (!text) return null;
  const into = scope === "catalog" ? fix.catalog : null;
  return {
    item: into?.item ?? null,
    path: into?.path ?? fix.path,
    value: text,
    source: null,
    at: null,
    note: null,
    min: null,
    max: null,
    ops: [],
  };
}

/** 選んだ部品を入れる編集。マヌーバのスラスタは `fune maneuver set` と同じ引数、設定の欄は `fune set` と同じ文字列 */
export type PickEdit = { text: SetText } | { command: CommandText };

/** 部品を選んだときに送るもの。選んでいない・マヌーバが設計に無いときは null */
export function pickEdit(fix: PickFix, part: string, design: DesignView): PickEdit | null {
  if (!part) return null;
  if (fix.target.kind === "maneuver") {
    const id = fix.target.id;
    const maneuver = design.maneuvers.find((x) => x.id === id);
    if (!maneuver) return null;
    return { command: { command: "maneuver_set", ...maneuver.text, thruster: part } };
  }
  return {
    text: {
      item: null,
      path: fix.path,
      value: part,
      source: null,
      at: null,
      note: null,
      min: null,
      max: null,
      ops: [],
    },
  };
}
