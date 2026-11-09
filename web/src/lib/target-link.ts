import type { DesignTarget } from "@/api/types";

/** 部品のページ。設計に無い部品（読み込めなかったファイルなど）は開けないので null */
function partPage(id: string, partIds: ReadonlySet<string>): string | null {
  return partIds.has(id) ? `/design/components/${id}` : null;
}

/**
 * 診断・不足入力の対象（API が構造で返す `DesignTarget`）→ 直しに行くページの URL。開くページが無ければ null。
 * 表示用のパスの文字列は分解しない。`partIds` は設計にある部品の ID。
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
      return target.target === "setting" ? "/project/settings" : partPage(target.part, partIds);
    case "any_part":
    case "group":
      return "/design/components";
    case "mode":
    case "modes":
      return "/design/modes";
    case "net":
    case "harness":
    case "view":
      return "/design/diagram";
    case "case":
      return "/mission/cases";
    case "catalog":
      return `/design/catalog/${encodeURIComponent(target.id)}`;
    case "attribute_def":
      return "/design/defs/attributes";
    case "kind_def":
      return "/design/defs/kinds";
    case "source":
    case "analysis":
      return null;
  }
}
