import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { OrbitPlaneTrack } from "@/api/types";
import { getT } from "@/i18n";
import { OrbitInset } from "./OrbitInset";

const plane: OrbitPlaneTrack = {
  x: [100, 0, -100],
  y: [0, 100, 0],
  eclipse: [false, false, true],
  sun_dir_body: [
    [1, 0, 0],
    [1, 0, 0],
    [1, 0, 0],
  ],
  nadir_body: [
    [0, 0, 1],
    [0, 0, 1],
    [0, 0, 1],
  ],
  body_radius: 50,
  sun_elevation: 0.3,
  shadow_half_width: 50,
};
const wrap = (el: React.ReactNode) =>
  renderToStaticMarkup(<QueryClientProvider client={new QueryClient()}>{el}</QueryClientProvider>);

describe("OrbitInset（描画）", () => {
  const I = getT().systems.thermalImage.orbit;
  it("日照の標本では日照と書き、日陰の標本では日陰と書いて衛星の点を薄くする", () => {
    const lit = wrap(<OrbitInset plane={plane} index={0} />);
    expect(lit).toContain(I.sunlit);
    expect(lit).toContain(I.label);
    expect(lit).toContain('data-eclipse="false"');
    const dark = wrap(<OrbitInset plane={plane} index={2} />);
    expect(dark).toContain(I.eclipse);
    expect(dark).toContain('data-eclipse="true"');
  });
  it("標本が無ければ何も出さない", () => {
    expect(wrap(<OrbitInset plane={{ ...plane, x: [], y: [] }} index={0} />)).toBe("");
  });
});
