/**
 * 一覧の枠の「既定から切り替えた」状態（たたんだ組立品・開いたグループなど）を、閲覧者ごと・プロジェクトごとに
 * localStorage へ保存する。保存先が使えない環境（プライベートブラウズ・保存の禁止）でも動くよう、読み書きは try/catch で包む。
 * 持つのは「既定の状態から切り替えた枠の ID」だけなので、既定（中が部品だけのグループはたたむ、ほかは開く）が変わる
 * 設計の編集（グループに組立品を入れる、など）のあとは、その枠は新しい既定から数える。
 */
import type { KeyValueStorage } from "@/shell/sidebar/sidebar-state";

const KEY_PREFIX = "iwafune.components.flipped.";

function storage(): KeyValueStorage | null {
  try {
    return typeof window === "undefined" ? null : window.localStorage;
  } catch {
    return null;
  }
}

/** 保存してある、切り替えた枠の ID（無い・読めないなら空） */
export function readFlipped(
  project: string,
  s: KeyValueStorage | null = storage(),
): ReadonlySet<string> {
  try {
    const v = s?.getItem(KEY_PREFIX + project);
    const parsed: unknown = v ? JSON.parse(v) : [];
    return new Set(
      Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === "string") : [],
    );
  } catch {
    return new Set();
  }
}

/** 切り替えた枠の ID を保存する。`known` に無い ID（消えた・名前の変わった枠）は落とす */
export function writeFlipped(
  project: string,
  ids: ReadonlySet<string>,
  known: (id: string) => boolean,
  s: KeyValueStorage | null = storage(),
): void {
  try {
    s?.setItem(KEY_PREFIX + project, JSON.stringify([...ids].filter(known).sort()));
  } catch {
    // 保存できなくても、たたむ・開くはできる
  }
}
