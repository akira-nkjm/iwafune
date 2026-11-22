import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { expect, it, vi } from "vitest";
import type { ThermalSteadyOutput, ThermalTransientOutput } from "@/api/types";
import { fixture } from "@/test/fixture";
import { ThermalImage } from "./ThermalImage";

const layout = vi.hoisted(() => ({ props: null as Record<string, unknown> | null }));
vi.mock("@/features/layout3d/LazyLayout3D", () => ({
  LazyLayout3D: (props: Record<string, unknown>) => {
    layout.props = props;
    return null;
  },
}));

const steady = { cases: [] } as unknown as ThermalSteadyOutput;
const transient = {
  series: {
    t: [0, 60],
    channels: [
      {
        key: "OBC",
        kind: "node_temperature",
        subject: "OBC",
        dimension: "Temperature",
        values: [250, 330],
      },
    ],
  },
  orbit_plane: {
    x: [1, 2],
    y: [0, 1],
    eclipse: [false, true],
    sun_dir_body: [
      [1, 0, 0],
      [0, 1, 0],
    ],
    nadir_body: [
      [0, 0, 1],
      [0, 0, 1],
    ],
    body_radius: 1,
    sun_elevation: 0,
    shadow_half_width: 1,
  },
} as unknown as ThermalTransientOutput;

it("温度の出どころが最悪のままなら、軌道の図も太陽の矢印も 3D に渡さない（非定常を選んだときだけ）", () => {
  renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>
      <ThermalImage design={fixture} steady={steady} transient={transient} heat={[]} />
    </QueryClientProvider>,
  );
  expect(layout.props).toMatchObject({ variant: "heat", sun: null });
  expect(layout.props?.corner ?? null).toBeNull();
});
