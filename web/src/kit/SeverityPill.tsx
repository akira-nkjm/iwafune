import type { Severity } from "@/api/types";
import { useT } from "@/i18n";
import { Pill } from "./Pill";

const tone = { error: "neg", warning: "warn", info: "neutral" } as const;

/** 診断の重さ */
export function SeverityPill({ severity }: { severity: Severity }) {
  const t = useT();
  return <Pill tone={tone[severity]}>{t.severity[severity]}</Pill>;
}
