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

// `Required` にするので、`EvidenceSource` に欄が増えて下書きが持たないままだと、型の誤りで気づく
// （置き換える Op で欄を黙って落とさないための確認。ダイアログが持たない欄は、保存のたびに消える）
const everyField: Required<EvidenceSource> = {
  id: "ds_all",
  kind: "datasheet",
  label: "All fields",
  ref: "https://example.com/all.pdf",
  revision: "Rev. A",
  retrieved: "2026-10-15",
  sha256: "0".repeat(64),
  distribution: "public",
  date: "2026-10-16",
  by: "tester",
  note: "note",
};

describe("setSourceOp", () => {
  it("keeps every field of a source the dialog edits", () => {
    expect(setSourceOp(draftOf(everyField))).toEqual({ op: "set_source", source: everyField });
  });

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
