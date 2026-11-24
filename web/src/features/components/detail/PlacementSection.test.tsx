import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, expect, it, vi } from "vitest";
import type { PartRow, PoseQuantities } from "@/api/types";
import { getT, setLang } from "@/i18n";
import { bodyAt, fixture, mountAt, qv, zeroPose } from "@/test/fixture";
import { initialForm, PlacementSection, placeText } from "./PlacementSection";

vi.mock("@/api/ops", () => ({
  useApplyOps: () => ({ mutate: vi.fn(), isPending: false }),
  useCommandOps: () => ({ ops: null, error: null }),
}));
vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
afterEach(() => setLang("ja"));

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

it("取付の根の欄は機体座標での `[pose]` から始まり、打った位置は取付先なしの配置になる", () => {
  const base = fixture.parts[0]!;
  const body = {
    ...bodyAt([-0.045, 0, 0], [1, 0, 0, 0], true),
    authored_position: { x: -45, y: 0, z: 0, unit: "mm", frame: null },
    quantities: {
      position: [mm(-45), mm(0), mm(0)] as PoseQuantities["position"],
      rotation: [deg(0, "0°"), deg(0, "0°"), deg(90, "90.0°")] as PoseQuantities["rotation"],
    },
  };
  const root: PartRow = { ...base, id: "FRAME", mount: null, body };
  const form = initialForm(root);
  expect(form).toMatchObject({
    on: "",
    point: "",
    position: "-45 0 0 mm",
    angles: ["0", "0", "90"],
  });
  // 位置だけ変えると、取付先は空（根のまま）で位置を送る。姿勢は書かれた値（無ければ送らない）
  const text = placeText(root, { ...form, position: "-30 0 0 mm" }, form);
  expect(text).toMatchObject({
    command: "part_place",
    part: "FRAME",
    on: "",
    position: "-30 0 0 mm",
    rotation: null,
  });
  const html = renderToStaticMarkup(<PlacementSection design={fixture} part={root} />);
  expect(html).toMatch(/aria-label="Z 軸まわりの角度（度）"[^>]*value="90"/);
});
