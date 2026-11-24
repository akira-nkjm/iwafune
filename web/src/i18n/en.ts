/** 英語の辞書。形は日本語の辞書（Dict）と同じ */
import { catalogEn } from "./catalog/en";
import { commonEn } from "./common/en";
import { componentsEn } from "./components/en";
import { conditionsEn } from "./conditions/en";
import { defsEn } from "./defs/en";
import { diagramEn } from "./diagram/en";
import { fieldsEn } from "./fields/en";
import { fixesEn } from "./fixes/en";
import type { Dict } from "./ja";
import { maneuversEn } from "./maneuvers/en";
import { modesEn } from "./modes/en";
import { overviewEn } from "./overview/en";
import { phasesEn } from "./phases/en";
import { projectEn } from "./project/en";
import { requirementsEn } from "./requirements/en";
import { shellEn } from "./shell/en";
import { sourcesEn } from "./sources/en";
import { systemsEn } from "./systems/en";

export const en: Dict = {
  ...commonEn,
  ...catalogEn,
  ...shellEn,
  ...componentsEn,
  ...modesEn,
  ...diagramEn,
  ...defsEn,
  ...systemsEn,
  ...fieldsEn,
  ...fixesEn,
  ...overviewEn,
  ...projectEn,
  ...conditionsEn,
  ...maneuversEn,
  ...phasesEn,
  ...requirementsEn,
  ...sourcesEn,
};
