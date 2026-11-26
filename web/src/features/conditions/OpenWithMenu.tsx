import { ChevronDown } from "lucide-react";
import { Link } from "react-router";
import type { CaseView, DefsView } from "@/api/types";
import { useT } from "@/i18n";
import { systemTitle } from "@/lib/design";
import { paths } from "@/lib/nav";
import { Button } from "@/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/ui/dropdown-menu";

/** ケースの行の「このケースで開く」。各系のページへのリンクを 1 つのメニューにまとめ、行を高くしない */
export function OpenWithMenu({ kase, defs }: { kase: CaseView; defs: DefsView }) {
  const m = useT().conditions;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button size="sm" variant="outline" aria-label={m.openWithCase(kase.id)}>
          {m.openWith}
          <ChevronDown size={13} />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {defs.systems.map((s) => (
          <DropdownMenuItem key={s} asChild>
            <Link to={paths.system(s, { kase: kase.id })}>{systemTitle(s, defs)}</Link>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
