import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { expect, it } from "vitest";
import { keys } from "@/api/keys";
import type { ModeHeader, ModeSequenceView, ModeTable, QuantityView } from "@/api/types";
import { getT } from "@/i18n";
import { fixture } from "@/test/fixture";
import { ModeTiming } from "./matrix/ModeTiming";
import { SequencePanel } from "./SequencePanel";

const q = (display: string) => ({ display }) as QuantityView;

const sequences: ModeSequenceView[] = [
  {
    name: "default",
    legacy: true,
    anchor: null,
    segments: [
      { mode: "NOMINAL", duration: q("76.0 min") },
      { mode: "DOWNLINK", duration: q("9.6 min") },
    ],
  },
  {
    name: "imaging_day",
    legacy: false,
    anchor: { from: "eclipse_exit", offset: q("5.0 min") },
    segments: [{ mode: "MISSION", duration: q("10.0 min") }],
  },
];

const table = (active: string | null): ModeTable => ({
  modes: [],
  rows: [],
  sequences,
  active_sequence: active,
});

function client() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  qc.setQueryData(keys.defs, fixture.defs);
  return qc;
}

const render = (node: React.ReactNode) =>
  renderToStaticMarkup(<QueryClientProvider client={client()}>{node}</QueryClientProvider>);

it("使う並びの区間と、使用中の印と、従来の書き方の注記を出す", () => {
  const t = getT().modes.sequences;
  const html = render(<SequencePanel table={table("default")} />);
  expect(html).toContain(`default (${t.inUse})`);
  expect(html).toContain("imaging_day");
  expect(html).toContain("76.0 min");
  expect(html).toContain(t.legacy);
  expect(html).toContain(t.anchorNone);
  // 使用中の並びを見ているときは「使う」ボタンが無効で、文言は使用中
  expect(html).toMatch(new RegExp(`<button[^>]*disabled[^>]*>${t.inUse}`));
});

it("使う並びが決まっていなければ最初の並びを見せ、並びが無ければ何も出さない", () => {
  expect(render(<SequencePanel table={table(null)} />)).toContain("76.0 min");
  expect(render(<SequencePanel table={{ ...table(null), sequences: [] }} />)).toBe("");
});

const mode = (extra: Partial<ModeHeader>): ModeHeader => ({
  id: "MISSION",
  label: "",
  description: null,
  file: "modes/MISSION.toml",
  max_duration: q("10.0 min"),
  when: null,
  start: null,
  ...extra,
});

it("モードのタイミングは未指定なら案内を、指定があれば要約を小さな文字で出す", () => {
  const t = getT().modes.timing;
  expect(render(<ModeTiming mode={mode({})} />)).toContain(t.unspecified);
  const set = render(
    <ModeTiming
      mode={mode({ when: "sunlit", start: { from: "eclipse_exit", offset: q("5.0 min") } })}
    />,
  );
  // 表示名の表が無いテストでは値の綴りのまま
  expect(set).toContain("sunlit");
  expect(set).toContain("eclipse_exit 5.0 min");
  expect(set).not.toContain(t.unspecified);
});
