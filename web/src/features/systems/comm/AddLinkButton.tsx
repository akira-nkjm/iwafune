import { Plus } from "lucide-react";
import { useState } from "react";
import type { DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { AddLinkDialog } from "@/ops/add-link/AddLinkDialog";
import { Button } from "@/ui/button";

/** 通信のページの「回線を足す」ボタン（ダイアログは `ops/add-link`。足りない入力の行からも同じものを開く） */
export function AddLinkButton({ design }: { design: DesignView }) {
  const tl = useT().conditions.links;
  const [adding, setAdding] = useState(false);
  return (
    <>
      <Button size="sm" onClick={() => setAdding(true)}>
        <Plus size={13} />
        {tl.add}
      </Button>
      {adding && <AddLinkDialog design={design} onOpenChange={setAdding} />}
    </>
  );
}
