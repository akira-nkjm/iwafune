import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { getT } from "@/i18n";
import { ModeList } from "./ModeShapes";
import type { ModeRow } from "./mode-shapes";

const shape = { index: 1, displacement: [], magnitude: [] };
const rows: ModeRow[] = [
  { index: 1, frequency: 70, classLabel: "局所", axisLabel: null, shape },
  {
    index: 2,
    frequency: 82,
    classLabel: "全体・横",
    axisLabel: "X 軸",
    shape: { ...shape, index: 2 },
  },
];

const html = (selected: number) =>
  renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>
      <ModeList rows={rows} selected={selected} onSelect={() => {}} />
    </QueryClientProvider>,
  );

describe("モードの選択肢", () => {
  it("振動数・種類・向きを API の値と表示名のまま並べ、選んだモードだけが選択状態", () => {
    const out = html(2);
    const t = getT().systems.modeShapes;
    expect(out).toContain(t.mode(1));
    expect(out).toContain(t.mode(2));
    expect(out).toContain("全体・横 · X 軸");
    // 向きの無いモードは種類だけ
    expect(out).toContain(">局所<");
    expect(out.match(/aria-checked="true"/g)).toHaveLength(1);
    expect(out.indexOf('aria-checked="true"')).toBeGreaterThan(out.indexOf(t.mode(2)) - 400);
  });

  it("どれも radiogroup の radio", () => {
    const out = html(1);
    expect(out).toContain('role="radiogroup"');
    expect(out.match(/role="radio"/g)).toHaveLength(2);
  });
});
