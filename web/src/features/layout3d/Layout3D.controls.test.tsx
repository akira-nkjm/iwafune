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
  hidden: new Set<string>() as ReadonlySet<string>,
  helpers: null as Record<string, boolean> | null,
  focus: undefined as unknown,
  home: 0,
  browser: vi.fn(),
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
    if (initial instanceof Set)
      return [
        state.hidden,
        (v: ReadonlySet<string>) => {
          state.hidden = v;
        },
      ];
    if (initial && typeof initial === "object" && "grid" in initial)
      return [
        state.helpers ?? initial,
        (v: Record<string, boolean>) => {
          state.helpers = v;
        },
      ];
    if (initial === 0)
      return [
        state.home,
        (v: unknown) => {
          state.home = typeof v === "function" ? v(state.home) : (v as number);
        },
      ];
    if (initial === undefined)
      return [
        state.focus,
        (v: unknown) => {
          state.focus = typeof v === "function" ? v(state.focus) : v;
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
vi.mock("./LayoutBrowser", () => ({
  LayoutBrowser: (props: unknown) => {
    state.browser(props);
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
      },
      body: {
        position: [0, 0, 0] as [number, number, number],
        orientation: [1, 0, 0, 0] as [number, number, number, number],
        root: false,
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
  state.hidden = new Set();
  state.helpers = null;
  state.focus = undefined;
  state.home = 0;
  state.scene.mockClear();
  state.browser.mockClear();
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

type BrowserProps = {
  selected: string | null;
  onSelect: (id: string) => void;
  onHidden: (hidden: Set<string>) => void;
};
type SceneProps = { boxes: { id: string }[]; selected: string | null; focus: unknown };

it("ツリーで隠した部品は場面に渡さず、ツリーで押した部品は選んで視点を寄せる", () => {
  const id = design.parts[0]!.id;
  render();
  const browser = () => state.browser.mock.lastCall![0] as BrowserProps;
  const scene = () => state.scene.mock.lastCall![0] as SceneProps;
  expect(scene().boxes.map((b) => b.id)).toEqual([id]);
  browser().onHidden(new Set([id]));
  render();
  expect(scene().boxes).toEqual([]);
  browser().onHidden(new Set());
  browser().onSelect(id);
  render();
  expect(scene()).toMatchObject({ selected: id, focus: { id, n: 1 } });
  expect(browser().selected).toBe(id);
  browser().onSelect(id);
  render();
  expect(scene().focus).toEqual({ id, n: 2 });
});

it("3D で選んだ部品はツリーでも選ばれ、視点は動かさない", () => {
  const id = design.parts[0]!.id;
  render();
  (state.scene.mock.lastCall![0] as { onSelect: (id: string) => void }).onSelect(id);
  render();
  expect((state.browser.mock.lastCall![0] as BrowserProps).selected).toBe(id);
  expect((state.scene.mock.lastCall![0] as SceneProps).focus).toBeNull();
});

it("座標の表示の切り替えを場面に渡す", () => {
  const L = getT().systems.layout3d;
  const html = render();
  for (const key of ["origin", "grid", "gizmo"] as const) expect(html).toContain(L.helper[key]);
  expect(state.scene.mock.lastCall?.[0]).toMatchObject({
    helpers: { origin: true, grid: true, gizmo: true },
  });
  state.helpers = { origin: false, grid: true, gizmo: false };
  render();
  expect(state.scene.mock.lastCall?.[0]).toMatchObject({ helpers: state.helpers });
});

it.each(["ja", "en"] as const)(
  "%s: ホームポジションのボタンを出し、押した回数を場面に渡す",
  (lang) => {
    setLang(lang);
    const label = getT().systems.layout3d.home;
    expect(render()).toContain(`aria-label="${label}"`);
    expect(state.scene.mock.lastCall?.[0]).toMatchObject({ home: 0 });
    state.home = 2;
    render();
    expect(state.scene.mock.lastCall?.[0]).toMatchObject({ home: 2 });
  },
);
