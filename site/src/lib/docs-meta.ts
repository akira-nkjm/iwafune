/**
 * 前書き（front matter）のない docs のページから、題名と説明を作る。
 * 題名は最初の `# H1`、説明は H1 のあとの最初の段落。
 */
import type { Heading, Paragraph, Root } from "mdast";
import { toString as nodeText } from "mdast-util-to-string";
import remarkMath from "remark-math";
import remarkParse from "remark-parse";
import { unified } from "unified";

/** ページの題名と説明（無いものは undefined）。 */
export interface DocsMeta {
  title?: string;
  description?: string;
}

const MAX_DESCRIPTION = 160;

/** 先頭の front matter（`---` で囲んだ部分）を取り除く。 */
export function stripFrontmatter(markdown: string): string {
  return markdown.replace(/^﻿?---\r?\n[\s\S]*?\r?\n---\r?\n?/, "");
}

/** 長い説明を文字数で切り、末尾に `…` を付ける。 */
function clip(text: string): string {
  const flat = text.replace(/\s+/g, " ").trim();
  if (flat.length <= MAX_DESCRIPTION) return flat;
  return `${flat.slice(0, MAX_DESCRIPTION - 1).trimEnd()}…`;
}

/** Markdown の本文から題名と説明を読む。 */
export function deriveMeta(markdown: string): DocsMeta {
  const tree = unified().use(remarkParse).use(remarkMath).parse(stripFrontmatter(markdown)) as Root;
  const nodes = tree.children;
  const h1 = nodes.findIndex((n) => n.type === "heading" && (n as Heading).depth === 1);
  if (h1 < 0) return {};
  const title = nodeText(nodes[h1] as Heading).trim();
  const meta: DocsMeta = title ? { title } : {};
  for (const node of nodes.slice(h1 + 1)) {
    if (node.type === "heading") break;
    if (node.type !== "paragraph") continue;
    const text = nodeText(node as Paragraph).trim();
    // 画像だけの段落は説明にしない。
    if (text !== "" && !(node as Paragraph).children.every((c) => c.type === "image")) {
      meta.description = clip(text);
      break;
    }
  }
  return meta;
}
