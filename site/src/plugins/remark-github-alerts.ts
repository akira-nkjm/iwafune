/**
 * GitHub の注意書き（`> [!NOTE]` など）を Starlight の囲み（aside）にする remark プラグイン。
 * 対応: NOTE, TIP, IMPORTANT, WARNING, CAUTION。
 */
import type { Blockquote, Paragraph, Root } from "mdast";
import type { Plugin } from "unified";
import { visit } from "unist-util-visit";

/** 種類ごとの題名と、Starlight の囲みの種類。 */
const KINDS: Record<string, { title: string; variant: "note" | "tip" | "caution" | "danger" }> = {
  NOTE: { title: "Note", variant: "note" },
  TIP: { title: "Tip", variant: "tip" },
  IMPORTANT: { title: "Important", variant: "note" },
  WARNING: { title: "Warning", variant: "caution" },
  CAUTION: { title: "Caution", variant: "danger" },
};

const remarkGithubAlerts: Plugin<[], Root> = () => (tree) => {
  visit(tree, "blockquote", (node: Blockquote) => {
    const first = node.children[0];
    if (first?.type !== "paragraph") return;
    const lead = first.children[0];
    if (lead?.type !== "text") return;
    const m = /^\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\][ \t]*\r?\n?/i.exec(lead.value);
    const kind = m?.[1] ? KINDS[m[1].toUpperCase()] : undefined;
    if (!m || !kind) return;
    lead.value = lead.value.slice(m[0].length);
    const body: Paragraph[] = first.children.length === 1 && lead.value === "" ? [] : [first];
    const content: Blockquote = {
      type: "blockquote",
      data: { hName: "div", hProperties: { className: ["starlight-aside__content"] } },
      children: [...body, ...node.children.slice(1)],
    };
    const title: Paragraph = {
      type: "paragraph",
      data: {
        hName: "p",
        hProperties: { className: ["starlight-aside__title"], "aria-hidden": "true" },
      },
      children: [{ type: "text", value: kind.title }],
    };
    node.data = {
      hName: "aside",
      hProperties: {
        className: ["starlight-aside", `starlight-aside--${kind.variant}`],
        ariaLabel: kind.title,
      },
    };
    node.children = [title, content];
  });
};

export default remarkGithubAlerts;
