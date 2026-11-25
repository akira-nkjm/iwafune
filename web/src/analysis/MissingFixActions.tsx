import { useState } from "react";
import type { DesignView, MissingInput } from "@/api/types";
import { useT } from "@/i18n";
import { AddCaseDialog } from "@/ops/add-case/AddCaseDialog";
import { AddModeDialog } from "@/ops/add-mode/AddModeDialog";
import { AddPartDialog } from "@/ops/add-part/AddPartDialog";
import { AddStationDialog } from "@/ops/add-station/AddStationDialog";
import { LinkDialog } from "@/ops/link/LinkDialog";
import { ManeuverDialog } from "@/ops/maneuver/ManeuverDialog";
import { emptyManeuverText } from "@/ops/maneuver/maneuver-form";
import { MissingPartDialog } from "./MissingPartDialog";
import { MissingPickDialog } from "./MissingPickDialog";
import { MissingValueDialog } from "./MissingValueDialog";
import { type RowAction, rowActions } from "./missing-fix";

const ACTION = "text-accent-ink hover:underline";

/** ダイアログを開く操作のボタンの文言（候補の種類で引く） */
function actionLabel(a: RowAction, t: ReturnType<typeof useT>["fixes"]): string {
  switch (a.kind) {
    case "value":
      return t.enterValue;
    case "pick":
      return t.pick.action[a.fix.what];
    case "part":
      return a.fix.kind === "assign_part" ? t.assignPart : t.setOnPart;
    case "add_part":
      return t.addPart;
    case "add_item":
      return t.addItem[a.list];
  }
}

/** 操作の並びの key（候補の種類と、候補が持つコマンドか行き先で一意） */
function keyOf(a: RowAction): string {
  switch (a.kind) {
    case "add_item":
      return `${a.kind}-${a.command}`;
    default:
      return `${a.kind}-${a.fix.command}`;
  }
}

/** 一覧の追加のダイアログ（各ページの追加ボタンと同じもの。足りない入力が ID を名指ししていれば、欄の初期値にする） */
function AddItemDialog({
  action,
  design,
  onOpenChange,
}: {
  action: Extract<RowAction, { kind: "add_item" }>;
  design: DesignView | undefined;
  onOpenChange: (open: boolean) => void;
}) {
  switch (action.list) {
    case "ground_station":
      return <AddStationDialog initialId={action.id} onOpenChange={onOpenChange} />;
    case "comm_link":
      return (
        design && <LinkDialog design={design} initialId={action.id} onOpenChange={onOpenChange} />
      );
    case "case":
      return (
        design && (
          <AddCaseDialog design={design} initialId={action.id} onOpenChange={onOpenChange} />
        )
      );
    case "mode":
      return design && <AddModeDialog table={design.modes} open onOpenChange={onOpenChange} />;
    case "maneuver":
      return (
        design && (
          <ManeuverDialog
            design={design}
            initial={{ ...emptyManeuverText(), id: action.id ?? "" }}
            adding
            onOpenChange={onOpenChange}
          />
        )
      );
  }
}

/**
 * 足りない入力の行の操作。API が返した直し方の候補（`MissingInput.fixes`）ごとに、その場のダイアログ（値を入れる・選ぶ）か、
 * 部品の追加・割り当てのダイアログ、一覧の追加のダイアログ（モード・マヌーバ・地上局・通信の回線・解析ケース）を出す。対象の文字列は読まない。
 */
export function MissingFixActions({
  input,
  design,
}: {
  input: MissingInput;
  design: DesignView | undefined;
}) {
  const t = useT().fixes;
  const [open, setOpen] = useState<RowAction | null>(null);
  const close = (o: boolean) => !o && setOpen(null);
  return (
    <>
      {rowActions(input).map((a) => {
        const key = keyOf(a);
        return (
          <button key={key} type="button" className={ACTION} onClick={() => setOpen(a)}>
            {actionLabel(a, t)}
          </button>
        );
      })}
      {open?.kind === "value" && <MissingValueDialog fix={open.fix} onOpenChange={close} />}
      {open?.kind === "part" && <MissingPartDialog fix={open.fix} onOpenChange={close} />}
      {open?.kind === "add_part" && design && (
        <AddPartDialog
          design={design}
          open
          onOpenChange={close}
          onDone={() => setOpen(null)}
          narrow={{ kinds: open.fix.kinds, catalogItems: open.fix.catalog_items }}
        />
      )}
      {open?.kind === "add_item" && (
        <AddItemDialog action={open} design={design} onOpenChange={close} />
      )}
      {open?.kind === "pick" && design && (
        <MissingPickDialog fix={open.fix} design={design} onOpenChange={close} />
      )}
    </>
  );
}
