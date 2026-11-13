import { expect, it } from "vitest";
import { fixture } from "@/test/fixture";
import { systemExtras } from "./extras";

it("構造の概要には配置リンクを出さない", () => {
  expect(systemExtras("structure", fixture, [])).toEqual({});
});
