import type { Verdict } from "@/api/types";
import { useT } from "@/i18n";
import { Pill } from "./Pill";
import { verdictTone } from "./verdict-tone";

/** 判定（OK / 注意 / 違反） */
export function VerdictPill({ verdict, text }: { verdict: Verdict; text?: string }) {
  const t = useT();
  return <Pill tone={verdictTone[verdict]}>{text ?? t.verdict[verdict]}</Pill>;
}
