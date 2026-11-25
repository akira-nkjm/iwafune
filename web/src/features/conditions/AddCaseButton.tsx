import { Plus } from "lucide-react";
import { useState } from "react";
import type { DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { AddCaseDialog } from "@/ops/add-case/AddCaseDialog";
import { Button } from "@/ui/button";

/** 解析ケースのページの「ケースを足す」ボタン（ダイアログは `ops/add-case`。足りない入力の行からも同じものを開く）。設計を読み込んでから押せる */
export function AddCaseButton({ design }: { design: DesignView | undefined }) {
  const tc = useT().conditions.addCase;
  const [adding, setAdding] = useState(false);
  return (
    <>
      <Button size="sm" disabled={!design} onClick={() => setAdding(true)}>
        <Plus size={13} />
        {tc.open}
      </Button>
      {adding && design && <AddCaseDialog design={design} onOpenChange={setAdding} />}
    </>
  );
}
