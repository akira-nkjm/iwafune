/**
 * 画面写真の目印（`<!-- screenshot: <route> — <description> -->`）を図に変える remark プラグイン。
 * `site/src/assets/shots/<slug>.png`（日本語のページは先に `<slug>.ja.png`）があればその画像を、無ければ説明を載せた枠を出す。
 * slug の規則は lib/shots.ts の `shotSlug`。
 */
import fs from "node:fs";
import path from "node:path";
import type { Image, Paragraph, Parent, PhrasingContent, Root } from "mdast";
import type { Plugin } from "unified";
import { visit } from "unist-util-visit";
import { parseMarker, shotFileName } from "../lib/shots.ts";
import { docsDir, SHOTS_DIR } from "../lib/site.ts";

export interface ScreenshotsOptions {
  docsDir?: string;
  shotsDir?: string;
  /** ファイルの有無の確かめ方（テスト用）。 */
  exists?: (file: string) => boolean;
}

/** 部品の入れ物（hast の要素にする段落）。 */
function box(
  hName: string,
  className: string,
  children: PhrasingContent[] | Paragraph[],
  extra: Record<string, unknown> = {},
): Paragraph {
  return {
    type: "paragraph",
    data: { hName, hProperties: { className: className.split(" "), ...extra } },
    children: children as PhrasingContent[],
  };
}

const remarkScreenshots: Plugin<[ScreenshotsOptions?], Root> = (options = {}) => {
  const docs = options.docsDir ?? docsDir();
  const shotsDir = options.shotsDir ?? SHOTS_DIR;
  const exists = options.exists ?? fs.existsSync;
  return (tree, file) => {
    const fromDocs = file.path ? path.relative(docs, file.path).split(path.sep).join("/") : "";
    const lang = fromDocs.startsWith("ja/") ? "ja" : "en";
    visit(tree, "html", (node, index, parent: Parent | undefined) => {
      if (!parent || index === undefined) return;
      const comment = /^<!--([\s\S]*?)-->$/.exec(node.value.trim());
      const marker = comment ? parseMarker(comment[1] ?? "") : null;
      if (!marker) return;
      const candidates = [shotFileName(marker.route, lang), shotFileName(marker.route, "en")];
      const found = candidates.find((name) => exists(path.join(shotsDir, name)));
      let figure: Paragraph;
      if (found && file.path) {
        const rel = path.relative(path.dirname(file.path), path.join(shotsDir, found));
        const url = rel.split(path.sep).join("/");
        const image: Image = {
          type: "image",
          url: url.startsWith(".") ? url : `./${url}`,
          alt: marker.description,
        };
        // 説明は代替テキストにだけ使う（見える説明文は、文書が目印の次の行に書く）。
        figure = box("figure", "shot", [image] as never);
      } else {
        figure = box(
          "figure",
          "shot shot--placeholder",
          [
            box("div", "shot__frame", [
              box("span", "shot__route", [{ type: "text", value: marker.route }]),
              box("span", "shot__description", [{ type: "text", value: marker.description }]),
            ] as never),
          ] as never,
          {},
        );
      }
      // 段落が目印だけなら、段落ごと置き換える。
      parent.children[index] = figure;
    });
  };
};

export default remarkScreenshots;
