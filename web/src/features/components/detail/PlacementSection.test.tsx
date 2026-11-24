import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, expect, it, vi } from "vitest";
import { q } from "@/api/queries";
import type { ConfigurationView, PartRow, PoseQuantities } from "@/api/types";
import { getT, setLang } from "@/i18n";
import { byLabel, findElement, fire } from "@/test/elements";
import { fixture, mountAt, qv, zeroPose } from "@/test/fixture";
import { PlacementOverrides, PlacementSection } from "./PlacementSection";

const state = vi.hoisted(() => ({
  mutate: vi.fn(),
  make: vi.fn(),
  form: undefined as unknown,
  invoke: false,
}));

vi.mock("react", async (original) => {
  const actual = await original<typeof import("react")>();
  const renderState = actual.useState;
  return {
    ...actual,
    useState: (initial: unknown) => {
      if (!state.invoke) return renderState(initial);
      if (typeof initial === "function") return [initial(), vi.fn()];
      if (initial && typeof initial === "object" && "angles" in initial)
        return [
          state.form ?? initial,
          (update: (form: unknown) => unknown) => {
            state.form = update(state.form ?? initial);
          },
        ];
      return [initial, vi.fn()];
    },
  };
});
vi.mock("@/api/ops", () => ({
  useApplyOps: () => ({ mutate: state.mutate, isPending: false }),
  useCommandOps: (command: unknown) => {
    state.make(command);
    return { ops: null, error: null };
  },
}));
vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
afterEach(() => {
  setLang("ja");
  state.invoke = false;
  state.form = undefined;
  state.mutate.mockClear();
  state.make.mockClear();
});

const deg = (value: number, display: string) => ({ ...qv(value, "deg", "Angle"), display });
const mm = (value: number) => qv(value, "mm", "Length");

/** 取付先 PANEL（取付点 SLOT・SIDE）に、SLOT から Y まわり 90° で付けた部品 */
function mounted(): { parent: PartRow; part: PartRow } {
  const base = fixture.parts[0]!;
  const slot: PoseQuantities = {
    position: [mm(0), mm(0), mm(12)],
    rotation: [deg(0, "0°"), deg(0, "0°"), deg(0, "0°")],
  };
  const parent: PartRow = {
    ...base,
    id: "PANEL",
    mount_points: [
      { name: "SLOT", pose: slot },
      { name: "SIDE", pose: zeroPose() },
    ],
  };
  const part: PartRow = {
    ...base,
    id: "SS",
    mount: {
      ...mountAt("PANEL"),
      point: "SLOT",
      point_pose: slot,
      relative: {
        position: [mm(0), mm(0), mm(0)],
        rotation: [deg(0, "0°"), deg(90, "90.0°"), deg(0, "0°")],
      },
    },
  };
  return { parent, part };
}

it.each(["ja", "en"] as const)(
  "%s: 配置のラベルは欄の上で折り返し、入力に固定幅を付けない",
  (lang) => {
    setLang(lang);
    const t = getT().components.placement;
    const html = renderToStaticMarkup(
      <PlacementSection design={fixture} part={fixture.parts[0]!} />,
    );
    for (const label of [t.mountOn, t.position, t.orientation, t.bodyFrame]) {
      expect(html).toContain(`>${label}</label>`);
    }
    expect(html).not.toContain("truncate");
    expect(html).not.toMatch(/w-(28|52)/);
    expect(html).toContain("wrap-anywhere");
    expect(html).toContain("grid min-w-0 content-start gap-1");
    expect(html).toContain(`aria-label="${t.position}"`);
    for (const axis of ["X", "Y", "Z"]) expect(html).toContain(`aria-label="${t.axisAngle(axis)}"`);
  },
);

it("取付点は取付先の一覧から選び、位置は同じ単位で、姿勢は角度で出す", () => {
  const t = getT().components.placement;
  const { parent, part } = mounted();
  const design = { ...fixture, parts: [parent, part] };
  const html = renderToStaticMarkup(<PlacementSection design={design} part={part} />);
  // 取付点の選択肢: 原点と、取付先の取付点（名前と位置）
  expect(html).toContain(`<option value="">${t.origin}</option>`);
  expect(html).toContain('<option value="SLOT" selected="">SLOT (0 mm, 0 mm, 12 mm)</option>');
  expect(html).toContain('<option value="SIDE">SIDE (0 m, 0 m, 0 m)</option>');
  // 取付点の位置（部品と同じ単位）と、取付点に対する姿勢の角度の欄（度）
  expect(html).toContain("(0 mm, 0 mm, 12 mm)");
  expect(html).toMatch(/aria-label="Y 軸まわりの角度（度）"[^>]*value="90"/);
  // 取付点からの位置が書かれていなければ、0（未記入）と分かるように出す
  expect(html).toContain(t.positionUnset);
});

it("書かれた取付点が取付先に無ければ、その名前を印付きで出す", () => {
  const t = getT().components.placement;
  const { parent, part } = mounted();
  const lost: PartRow = { ...part, mount: { ...part.mount!, point: "GONE" } };
  const html = renderToStaticMarkup(
    <PlacementSection design={{ ...fixture, parts: [parent, lost] }} part={lost} />,
  );
  expect(html).toContain(`<option value="GONE" selected="">${t.missingPoint("GONE")}</option>`);
});

it.each([undefined, "STOWED"])("配置欄の変更は表示中の形態 %s を使う", (configuration) => {
  const { part, parent } = mounted();
  const t = getT().components.placement;
  const design = { ...fixture, parts: [part, parent] };
  state.invoke = true;
  fire(byLabel(PlacementSection({ design, part, configuration }), t.position), "onChange", {
    target: { value: "0 0 12 mm" },
  });
  PlacementSection({ design, part, configuration });
  expect(state.make).toHaveBeenLastCalledWith({
    command: "part_place",
    part: part.id,
    position: "0 0 12 mm",
    rotation: null,
    orientation: null,
    ...(configuration ? { configuration } : { on: parent.id, point: "SLOT" }),
  });
});

it("形態では取付を固定し、上書きがある部品の姿勢だけを基準に戻せる", () => {
  const { part, parent } = mounted();
  const overridden = { ...part, mount: { ...part.mount!, overridden: true } };
  const design = { ...fixture, parts: [overridden, parent] };
  const t = getT().components.placement;
  const html = renderToStaticMarkup(
    <PlacementSection design={design} part={overridden} configuration="STOWED" />,
  );
  expect(html).toContain(t.overridden);
  expect(html).toContain(t.resetOverride);
  expect(html).toMatch(new RegExp(`aria-label="${t.mountPart}"[^>]*disabled`));
  expect(html).toMatch(new RegExp(`aria-label="${t.point}"[^>]*disabled`));
  for (const [p, configuration] of [
    [part, "STOWED"],
    [overridden, undefined],
  ] as const) {
    const unchanged = renderToStaticMarkup(
      <PlacementSection design={design} part={p} configuration={configuration} />,
    );
    expect(unchanged).not.toContain(t.overridden);
    expect(unchanged).not.toContain(t.resetOverride);
  }
  state.invoke = true;
  const reset = findElement(
    PlacementSection({ design, part: overridden, configuration: "STOWED" }),
    (props) => props.children === t.resetOverride,
  )!;
  fire(reset, "onClick");
  expect(state.mutate).toHaveBeenCalledExactlyOnceWith({
    ops: [{ op: "unset_configuration_pose", configuration: "STOWED", part: part.id }],
    summary: t.resetSummary(part.id),
  });
});

it("部品詳細は上書きのある形態の取付点からの姿勢を読み取り専用で並べる", () => {
  const { part } = mounted();
  const stowed: ConfigurationView = {
    id: "STOWED",
    label: "Stowed",
    description: null,
    file: "configurations/STOWED.toml",
    text: { id: "STOWED" },
    overridden: [part.id],
  };
  const design = {
    ...fixture,
    configurations: [stowed, { ...stowed, id: "OTHER", overridden: [] }],
  };
  const client = new QueryClient();
  client.setQueryData(q.part(part.id, [], "STOWED").queryKey, {
    part: { ...part, mount: { ...part.mount!, overridden: true } },
    sections: [],
    modes: {},
    harnesses: [],
    diagnostics: [],
  });
  const html = renderToStaticMarkup(
    <QueryClientProvider client={client}>
      <PlacementOverrides design={design} part={part} />
    </QueryClientProvider>,
  );
  client.clear();
  expect(html).toContain("Stowed (STOWED)");
  expect(html).toContain("90.0°");
  expect(html).not.toContain("OTHER");
  expect(html).not.toContain("<input");
});

it("形態を選んでも取付の根の姿勢は編集できない", () => {
  const t = getT().components.placement;
  const part = { ...fixture.parts[0]!, mount: null };
  const html = renderToStaticMarkup(
    <PlacementSection design={fixture} part={part} configuration="STOWED" />,
  );
  expect(html).toContain(t.poseLocked);
  expect(html).toMatch(new RegExp(`aria-label="${t.position}"[^>]*disabled`));
});
