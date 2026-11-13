import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { getT, setLang } from "@/i18n";
import { fixture } from "@/test/fixture";
import { Layout3D } from "./Layout3D";
import { DEFAULT_STRUCTURE_OPACITY } from "./scene";

const state = vi.hoisted(() => ({
  axes: true,
  opacity: 0.3,
  selected: null as string | null,
  toggle: (_value: boolean) => {},
  slide: (_value: number) => {},
  scene: vi.fn(),
}));
vi.mock("react", async (original) => ({
  ...(await original<typeof import("react")>()),
  useState: (initial: unknown) => {
    if (initial === true)
      return [
        state.axes,
        (v: boolean) => {
          state.axes = v;
        },
      ];
    if (initial === 0.3)
      return [
        state.opacity,
        (v: number) => {
          state.opacity = v;
        },
      ];
    if (initial === null)
      return [
        state.selected,
        (v: string | null) => {
          state.selected = v;
        },
      ];
    return [initial, vi.fn()];
  },
}));
vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
vi.mock("@/api/ops", () => ({ usePlaceBody: () => ({ mutate: vi.fn() }) }));
vi.mock("@/features/components/detail/PlacementSection", () => ({ PlacementSection: () => null }));
vi.mock("@/kit/selection/Switch", () => ({
  Switch: ({ onChange, children }: { onChange: typeof state.toggle; children: string }) => {
    state.toggle = onChange;
    return <button type="button">{children}</button>;
  },
}));
vi.mock("@/kit/forms/RangeInput", () => ({
  RangeInput: ({ onChange }: { onChange: typeof state.slide }) => {
    state.slide = onChange;
    return null;
  },
}));
vi.mock("./Scene3D", () => ({
  Scene3D: (props: unknown) => {
    state.scene(props);
    return null;
  },
}));

const design = {
  ...fixture,
  parts: [
    {
      ...fixture.parts[0]!,
      system: "structure" as const,
      mount: {
        on: "",
        point: null,
        position: [0, 0, 0] as [number, number, number],
        authored_position: null,
        orientation: [1, 0, 0, 0] as [number, number, number, number],
        body_position: [0, 0, 0] as [number, number, number],
        body_orientation: [1, 0, 0, 0] as [number, number, number, number],
      },
    },
  ],
};
const render = () =>
  renderToStaticMarkup(<Layout3D variant="placement" design={design} com={null} />);

beforeEach(() => {
  state.axes = true;
  state.opacity = DEFAULT_STRUCTURE_OPACITY;
  state.selected = null;
  state.scene.mockClear();
});
afterEach(() => setLang("ja"));

it.each(["ja", "en"] as const)("%s: 凡例だけを切り替え、操作の帯は同じ位置と内容を保つ", (lang) => {
  setLang(lang);
  const legend = getT().systems.layout3d.axesLegend;
  const on = render();
  expect(on).toContain(legend);
  expect(on).not.toContain("<h2");
  state.toggle(false);
  const off = render();
  expect(off).not.toContain(legend);
  const toolbar = (html: string) => html.slice(0, html.indexOf('<div class="grid min-w-0'));
  expect(toolbar(off)).toBe(toolbar(on));
  expect(state.scene.mock.lastCall?.[0]).toMatchObject({ axes: null });
});

it("スライダーの変更を場面に渡し、選んだ部品を編集対象として保つ", () => {
  render();
  expect(state.scene.mock.lastCall?.[0]).toMatchObject({ structureOpacity: 0.3 });
  const { onSelect } = state.scene.mock.lastCall![0] as { onSelect: (id: string) => void };
  onSelect(design.parts[0]!.id);
  for (const opacity of [0, 0.5, 1]) {
    state.slide(opacity);
    const html = render();
    expect(state.scene.mock.lastCall?.[0]).toMatchObject({
      structureOpacity: opacity,
      selected: design.parts[0]!.id,
    });
    expect(html).toContain(getT().systems.layout3d.placementOf(design.parts[0]!.id));
  }
});
