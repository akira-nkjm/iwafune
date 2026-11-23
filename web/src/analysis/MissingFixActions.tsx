import { useState } from "react";
import { Link } from "react-router";
import type { DesignView, MissingInput } from "@/api/types";
import { useT } from "@/i18n";
import { MissingPickDialog } from "./MissingPickDialog";
import { MissingValueDialog } from "./MissingValueDialog";
import { type RowAction, rowActions } from "./missing-fix";

const ACTION = "text-accent-ink hover:underline";

/**
 * 足りない入力の行の操作。API が返した直し方の候補（`MissingInput.fixes`）ごとに、その場のダイアログ（値を入れる・部品を選ぶ）か、
 * 既存のページへのリンク（部品・一覧の追加）を出す。対象の文字列は読まない。
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
  const label = (a: RowAction & { kind: "link" }): string => {
    if (typeof a.label !== "string") return t.addItem[a.label.addItem];
    switch (a.label) {
      case "setOnPart":
        return t.setOnPart;
      case "addPart":
        return t.addPart;
      case "assignPart":
        return t.assignPart;
    }
  };
  return (
    <>
      {rowActions(input).map((a) => {
        const key =
          a.kind === "link"
            ? `${a.to}-${typeof a.label === "string" ? a.label : a.label.addItem}`
            : `${a.kind}-${a.fix.command}`;
        if (a.kind === "link")
          return (
            <Link key={key} to={a.to} className={ACTION}>
              {label(a)}
            </Link>
          );
        return (
          <button key={key} type="button" className={ACTION} onClick={() => setOpen(a)}>
            {a.kind === "value" ? t.enterValue : t.pickPart}
          </button>
        );
      })}
      {open?.kind === "value" && <MissingValueDialog fix={open.fix} onOpenChange={close} />}
      {open?.kind === "pick" && design && (
        <MissingPickDialog fix={open.fix} design={design} onOpenChange={close} />
      )}
    </>
  );
}
