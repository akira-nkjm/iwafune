import { expect, it } from "vitest";
import { keepsPromotedLayerResolution } from "./browser";

const SAFARI =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15";
const CHROME =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const EDGE = `${CHROME} Edg/131.0.0.0`;
const FIREFOX =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:133.0) Gecko/20100101 Firefox/133.0";
const IOS_CHROME =
  "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/131.0 Mobile/15E148 Safari/604.1";

it("Safari と iOS のブラウザは、絵にした解像度のまま引き伸ばす側。Chromium 系と Firefox は違う", () => {
  expect(keepsPromotedLayerResolution(SAFARI)).toBe(true);
  expect(keepsPromotedLayerResolution(IOS_CHROME)).toBe(true);
  expect(keepsPromotedLayerResolution(CHROME)).toBe(false);
  expect(keepsPromotedLayerResolution(EDGE)).toBe(false);
  expect(keepsPromotedLayerResolution(FIREFOX)).toBe(false);
});
