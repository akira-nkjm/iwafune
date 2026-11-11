import { useEnumLabel } from "@/api/labels";
import type { Severity } from "@/api/types";
import { Pill, type PillTone } from "./Pill";

const tone: Record<Severity, PillTone> = { error: "neg", warning: "warn", info: "neutral" };

/** 診断の重さ。名前は API の語彙（`labels.enum.severity`。CLI と同じ表） */
export function SeverityPill({ severity }: { severity: Severity }) {
  const label = useEnumLabel();
  return <Pill tone={tone[severity]}>{label("severity", severity)}</Pill>;
}
