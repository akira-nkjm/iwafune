import type { DesignTarget } from "@/api/types";
import { paths } from "./nav";

/** 部品のページ。設計に無い部品（読み込めなかったファイルなど）は開けないので null */
function partPage(id: string, partIds: ReadonlySet<string>): string | null {
  return partIds.has(id) ? paths.component(id) : null;
}

/**
 * 診断・不足入力の対象（API が構造で返す `DesignTarget`）→ 直しに行くページの URL。開くページが無ければ null。
 * 表示用のパスの文字列は分解しない。`partIds` は設計にある部品の ID。URL は `lib/nav.ts` の `paths` で作る。
 */
export function targetLink(
  target: DesignTarget | null,
  partIds: ReadonlySet<string>,
): string | null {
  if (!target) return null;
  switch (target.kind) {
    case "part":
      return partPage(target.id, partIds);
    case "port":
    case "mount":
    case "pose":
      return partPage(target.part, partIds);
    case "param":
      return target.target === "setting" ? paths.settings() : partPage(target.part, partIds);
    case "any_part":
    case "group":
      return paths.components();
    case "mode":
    case "modes":
      return paths.modes();
    case "maneuver":
      return paths.maneuver(target.id);
    case "maneuvers":
      return paths.maneuvers();
    case "requirement":
    case "criterion":
      return paths.requirements("list");
    case "net":
    case "harness":
    case "view":
      return paths.diagram();
    case "case":
      return paths.cases();
    case "phase":
      return paths.phase(target.id);
    case "catalog":
      return paths.catalogItem(target.id);
    case "attribute_def":
      return paths.defs("attributes");
    case "kind_def":
      return paths.defs("kinds");
    case "source":
      return paths.sources();
    case "analysis":
      return null;
  }
}
