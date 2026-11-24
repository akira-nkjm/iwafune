import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, expect, it, vi } from "vitest";
import type { AnalysisRunResult } from "@/api/types";
import { getT } from "@/i18n";
import { findingHex, toneHexFor } from "@/lib/tone";
import { bodyAt, fixture, mountAt } from "@/test/fixture";
import { Layout3D } from "./Layout3D";
import type { Box3D } from "./scene";

const state = vi.hoisted(() => ({
  on: false,
  boxes: [] as { id: string; color: string }[],
  pick: null as ((id: string) => void) | null,
}));
// 3D の場面は描かない。所見の表示を入れたとき（`useState(false)` がすべて真になる）の状態だけを見る
vi.mock("react", async (original) => ({
  ...(await original<typeof import("react")>()),
  useState: (initial: unknown) => [initial === false ? state.on : initial, vi.fn()],
}));
vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
vi.mock("@/api/ops", () => ({ usePlaceBody: () => ({ mutate: vi.fn() }) }));
vi.mock("@/features/components/detail/PlacementSection", () => ({ PlacementSection: () => null }));
vi.mock("./LayoutBrowser", () => ({ LayoutBrowser: () => null }));
vi.mock("./Scene3D", () => ({
  Scene3D: ({ boxes }: { boxes: Box3D[] }) => {
    state.boxes = boxes.map((b) => ({ id: b.id, color: b.color }));
    return null;
  },
}));

const [first, second, third] = fixture.parts;
const part = (p: NonNullable<typeof first>, at: number) => ({
  ...p,
  id: p.id,
  system: "structure" as const,
  mount: mountAt(""),
  body: bodyAt([at, 0, 0], [1, 0, 0, 0]),
});
const design = { ...fixture, parts: [part(first!, 0), part(second!, 1), part(third!, 2)] };
const [a, b, c] = design.parts.map((p) => p.id);

const findings = [
  {
    id: "structure.interference",
    status: "ok",
    output: {
      overlaps: [
        { first: a, second: b, acknowledged: false },
        { first: a, second: c, acknowledged: true, reason: "Hinge edge" },
      ],
    },
    part_findings: [
      { part: a, state: "open" },
      { part: b, state: "open" },
      { part: c, state: "acknowledged" },
    ],
  },
] as unknown as AnalysisRunResult[];

const render = (withFindings = true) =>
  renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>
      <Layout3D
        variant="placement"
        design={design}
        com={null}
        findings={withFindings ? findings : undefined}
      />
    </QueryClientProvider>,
  );

beforeEach(() => {
  state.on = false;
  state.boxes = [];
});

it("所見の表示の切り替えは所見を渡したときだけ出し、最初は切れていて、部品は系の色のまま・一覧も出ない", () => {
  const L = getT().systems.layout3d.findings;
  const html = render();
  expect(html).toContain(L.toggle);
  expect(html).not.toContain(`aria-label="${L.list}"`);
  const system = toneHexFor("light").blue;
  expect(
    state.boxes.map((x) => x.color).every((color) => color !== findingHex("light", "open")),
  ).toBe(true);
  expect(system).toBeTruthy();
  expect(render(false)).not.toContain(L.toggle);
});

it("入れると、所見に関わる部品を API の状態の色で塗り、ほかの部品は灰色にし、所見の一覧に部品を選ぶボタンを出す", () => {
  state.on = true;
  const L = getT().systems.layout3d.findings;
  const html = render();
  const color = Object.fromEntries(state.boxes.map((x) => [x.id, x.color]));
  expect(color[a!]).toBe(findingHex("light", "open"));
  expect(color[b!]).toBe(findingHex("light", "open"));
  expect(color[c!]).toBe(findingHex("light", "acknowledged"));
  expect(html).toContain(`aria-label="${L.list}"`);
  for (const id of [a, b, c]) expect(html).toContain(`aria-label="${L.select(id!)}"`);
  expect(html).toContain("Hinge edge");
});
