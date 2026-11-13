import { describe, expect, it } from "vitest";
import type { EvidenceSource } from "@/api/types";
import { draftOf, setSourceOp } from "./sources";

const source: EvidenceSource = {
  id: "ds_rw",
  kind: "datasheet",
  label: "RW",
  ref: "https://example.com/rw.pdf",
  revision: "Rev. C",
  retrieved: "2026-10-15",
  sha256: null,
  distribution: "on_request",
};

describe("setSourceOp", () => {
  it("round-trips a source through the draft and drops empty fields", () => {
    const op = setSourceOp(draftOf(source));
    expect(op).toEqual({
      op: "set_source",
      source: {
        ...source,
        date: null,
        by: null,
        note: null,
      },
    });
  });

  it("trims typed text and needs an ID", () => {
    const draft = { ...draftOf(null), id: " ds_new ", sha256: "  ", note: " 架空 " };
    const op = setSourceOp(draft);
    expect(op?.op === "set_source" && op.source).toMatchObject({
      id: "ds_new",
      kind: "datasheet",
      sha256: null,
      note: "架空",
      distribution: null,
    });
    expect(setSourceOp(draftOf(null))).toBeNull();
  });
});
