import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it } from "vitest";
import type { AttrInput, AttributeDefView, PartRow, ValueView } from "@/api/types";
import { getT, setLang } from "@/i18n";
import { fixture } from "@/test/fixture";
import { changedChoice, INPUT_FIELDS } from "./AttrInputFields";
import { AttrValueEditor } from "./AttrValueEditor";

afterEach(() => setLang("ja"));

const part = fixture.parts[0] as PartRow;
const label = "RW_X.attr";

const view = (value: ValueView["value"], origin: ValueView["origin"] = "override"): ValueView => ({
  attr: "attr",
  value,
  display: "",
  origin,
  evidence: "datasheet",
  source: null,
  note: null,
  file: null,
  line: null,
});

const def = (input: AttrInput): AttributeDefView => ({
  ...(fixture.defs.attributes[0] as AttributeDefView),
  id: "attr",
  input,
});

const choices = [
  { value: "xenon", label: "キセノン", label_en: "Xenon" },
  { value: "krypton", label: "krypton", label_en: null },
];

/** 欄の HTML（表のセルとして描く。入力の種類で欄が決まるかを見る） */
function cell(input: AttrInput, value: ValueView | undefined) {
  return renderToStaticMarkup(
    <AttrValueEditor
      layout="cell"
      owner={{ ...part, paths: { attr: { "": label } } }}
      attr="attr"
      def={def(input)}
      value={value}
      onCommit={() => {}}
    />,
  );
}

describe("属性の値の欄（入力の種類ごと）", () => {
  it("選ぶ: 候補を選択肢にして今の値を選び、日英の表示名を出す", () => {
    const html = cell({ kind: "choice", choices }, view({ type: "enum", value: "xenon" }));
    expect(html).toContain("<select");
    expect(html).toContain('<option value="xenon" selected="">キセノン (xenon)</option>');
    expect(html).toContain('<option value="krypton">krypton</option>');
    setLang("en");
    expect(cell({ kind: "choice", choices }, view({ type: "enum", value: "xenon" }))).toContain(
      "Xenon (xenon)",
    );
  });

  it("選ぶ: 真偽は true / false の候補から選ぶ", () => {
    const html = cell(
      {
        kind: "choice",
        choices: ["true", "false"].map((value) => ({ value, label: value, label_en: null })),
      },
      view({ type: "bool", value: false }),
    );
    expect(html).toContain('<option value="false" selected="">false</option>');
  });

  it("未設定なら空の選択肢（未設定）を出し、今の値が候補に無くても見失わない", () => {
    const t = getT().quantity;
    expect(cell({ kind: "choice", choices }, undefined)).toContain(
      `<option value="" selected="">${t.unsetChoice}</option>`,
    );
    const stale = cell({ kind: "choice", choices }, view({ type: "enum", value: "argon" }));
    expect(stale).toContain('<option value="argon" selected="">argon</option>');
    expect(stale).not.toContain('value=""');
  });

  it("部品を選ぶ: 候補は API が絞って返したもの。無ければ選べず、足すコマンドを案内する", () => {
    const parts = [{ value: "XE_TANK", label: "Xenon tank", label_en: null }];
    const reference = (list: typeof parts): AttrInput => ({
      kind: "reference",
      what: "part",
      roles: ["propellant_tank"],
      choices: list,
      add_command: "fune part add <ID> --kind <kind>",
    });
    const html = cell(reference(parts), view({ type: "text", value: "XE_TANK" }));
    expect(html).toContain('<option value="XE_TANK" selected="">Xenon tank (XE_TANK)</option>');
    const none = cell(reference([]), undefined);
    expect(none).toContain("disabled");
    expect(none).toContain(getT().quantity.noCandidates);
    expect(none).toContain('title="fune part add &lt;ID&gt; --kind &lt;kind&gt;"');
  });

  it("打つ: 今の値の入った文字の欄", () => {
    const html = cell({ kind: "text" }, view({ type: "enum", value: "xenon" }));
    expect(html).toContain(`aria-label="${label}"`);
    expect(html).toContain('value="xenon"');
    expect(html).not.toContain("<select");
  });

  it("量: 量の欄（今の表示のボタン）。直せない型は表示だけで、理由は型ごとの説明", () => {
    const quantity = cell({ kind: "quantity", dimension: "Mass", by_state: false }, undefined);
    expect(quantity).toContain(`aria-label="${getT().quantity.edit(label)}"`);
    const vector = cell({ kind: "read_only", reason: "vector" }, view({ type: "unset" }));
    expect(vector).toContain(getT().quantity.readOnly.vector);
    expect(vector).not.toContain("<input");
    expect(vector).not.toContain("<select");
  });

  it("定義の無い属性は表示だけ", () => {
    const html = renderToStaticMarkup(
      <AttrValueEditor
        layout="cell"
        owner={part}
        attr="zz"
        def={undefined}
        value={view({ type: "text", value: "abc" })}
        onCommit={() => {}}
      />,
    );
    expect(html).not.toContain("<input");
    expect(html).not.toContain("<select");
  });

  it("入力の種類の表は、量以外のすべての種類の欄を持つ", () => {
    expect(Object.keys(INPUT_FIELDS).sort()).toEqual(["choice", "read_only", "reference", "text"]);
  });
});

describe("確定の送り方", () => {
  it("選び直したときだけ、選んだ文字列を送る（今と同じ値・空の選択肢には送らない）", () => {
    expect(changedChoice("xenon", "krypton")).toBe("krypton");
    expect(changedChoice("xenon", "xenon")).toBeNull();
    expect(changedChoice("xenon", "")).toBeNull();
    expect(changedChoice("", "xenon")).toBe("xenon");
  });
});
