/**
 * 日本語の辞書（既定）。`Dict` はこの形から作るので、英語の辞書は同じキーと形を持つ（欠けると型のエラー）。
 * 値は文字列か、差し込みのある文言の小さな関数。区分ごとのファイル（<区分>/ja.ts）を束ねる。
 */
import { catalogJa } from "./catalog/ja";
import { commonJa } from "./common/ja";
import { componentsJa } from "./components/ja";
import { conditionsJa } from "./conditions/ja";
import { defsJa } from "./defs/ja";
import { diagramJa } from "./diagram/ja";
import { fieldsJa } from "./fields/ja";
import { modesJa } from "./modes/ja";
import { overviewJa } from "./overview/ja";
import { projectJa } from "./project/ja";
import { shellJa } from "./shell/ja";
import { sourcesJa } from "./sources/ja";
import { systemsJa } from "./systems/ja";

export const ja = {
  ...commonJa,
  ...catalogJa,
  ...shellJa,
  ...componentsJa,
  ...modesJa,
  ...diagramJa,
  ...defsJa,
  ...systemsJa,
  ...fieldsJa,
  ...overviewJa,
  ...projectJa,
  ...conditionsJa,
  ...sourcesJa,
};

export type Dict = typeof ja;
