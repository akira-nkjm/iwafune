import { useState } from "react";
import { useCommandOps } from "@/api/ops";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { TextInput } from "@/kit/forms/TextInput";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { sharedCatalogAddCommand } from "./catalog";

/**
 * 共有カタログを fune.toml に足す（`fune catalog source add` と同じ引数を打った文字列のまま service に渡して
 * AddCatalogSource にする）。名前の規則・重複・空の URL はサーバー（dry_run）が検査する。足すだけで取得はしない。
 */
export function AddSharedCatalogDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const m = useT().catalog.shared;
  const t = useT();
  const [name, setName] = useState("");
  const [git, setGit] = useState("");
  const [rev, setRev] = useState("");
  const made = useCommandOps(open ? sharedCatalogAddCommand(name, git, rev) : null);
  const failed = useOpCheck(open ? made.ops : null).failed;
  const error = made.error;
  return (
    <OpDialog
      open={open}
      onOpenChange={onOpenChange}
      title={m.add}
      description={m.addDesc}
      ops={made.ops}
      summary={m.summaryAdd(name)}
      confirmLabel={t.common.add}
    >
      <Field label={m.name} htmlFor="shared-name" hint={m.nameHint} error={error}>
        <TextInput
          id="shared-name"
          mono
          value={name}
          invalid={failed || !!error}
          placeholder="lab"
          onChange={(e) => setName(e.target.value)}
        />
      </Field>
      <Field label={m.git} htmlFor="shared-git" hint={m.gitHint}>
        <TextInput
          id="shared-git"
          mono
          value={git}
          placeholder="https://example.com/catalog.git"
          onChange={(e) => setGit(e.target.value)}
        />
      </Field>
      <Field label={m.revLabel} htmlFor="shared-rev" hint={m.revHint}>
        <TextInput
          id="shared-rev"
          mono
          value={rev}
          placeholder="main"
          onChange={(e) => setRev(e.target.value)}
        />
      </Field>
    </OpDialog>
  );
}
