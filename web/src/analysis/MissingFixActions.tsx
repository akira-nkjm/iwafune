import { useState } from "react";
import { Link } from "react-router";
import type { DesignView, MissingInput } from "@/api/types";
import { useT } from "@/i18n";
import { AddPartDialog } from "@/ops/add-part/AddPartDialog";
import { MissingPartDialog } from "./MissingPartDialog";
import { MissingPickDialog } from "./MissingPickDialog";
import { MissingValueDialog } from "./MissingValueDialog";
import { type RowAction, rowActions } from "./missing-fix";

const ACTION = "text-accent-ink hover:underline";

/** ダイアログを開く操作のボタンの文言（候補の種類で引く） */
function actionLabel(
  a: Exclude<RowAction, { kind: "link" }>,
  t: ReturnType<typeof useT>["fixes"],
): string {
  switch (a.kind) {
    case "value":
      return t.enterValue;
    case "pick":
      return t.pick.action[a.fix.what];
    case "part":
      return a.fix.kind === "assign_part" ? t.assignPart : t.setOnPart;
    case "add_part":
      return t.addPart;
  }
}

/**
 * 足りない入力の行の操作。API が返した直し方の候補（`MissingInput.fixes`）ごとに、その場のダイアログ（値を入れる・選ぶ）か、
 * 部品の追加・割り当てのダイアログ、既存のページへのリンク（一覧の追加）を出す。対象の文字列は読まない。
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
        const key = a.kind === "link" ? `${a.to}-${a.label.addItem}` : `${a.kind}-${a.fix.command}`;
        if (a.kind === "link")
          return (
            <Link key={key} to={a.to} className={ACTION}>
              {t.addItem[a.label.addItem]}
            </Link>
          );
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
      {open?.kind === "pick" && design && (
        <MissingPickDialog fix={open.fix} design={design} onOpenChange={close} />
      )}
    </>
  );
}
