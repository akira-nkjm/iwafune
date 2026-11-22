import { Languages } from "lucide-react";
import { type Lang, langNames, langs, setLang, useLang, useT } from "@/i18n";
import { Segmented } from "@/kit/selection/Segmented";
import { cn } from "@/lib/cn";

/**
 * 表示言語の切り替え（日本語 / English）。選んだ言語は閲覧者ごとに保存する。
 * `row` はサイドバーの下の欄に置く形で、ほかの行（ブランチ・変更履歴）と同じくアイコンと字だけにする（枠や地を付けない）。
 * `rail` は畳んだサイドバーの形で、1 つのボタンで交互に切り替える。どちらでもなければ、パネルの見出しなどに置く切り替え（`Segmented`）。
 */
export function LanguageSwitch({
  variant = "segmented",
}: {
  variant?: "segmented" | "row" | "rail";
}) {
  const lang = useLang();
  const t = useT();
  if (variant === "rail") {
    const next: Lang = lang === "ja" ? "en" : "ja";
    return (
      <button
        type="button"
        onClick={() => setLang(next)}
        title={`${t.common.language}: ${langNames[lang]} → ${langNames[next]}`}
        aria-label={`${t.common.language}: ${langNames[next]}`}
        className="mx-auto flex size-8 items-center justify-center rounded-control text-ink-3 hover:bg-panel hover:text-ink"
      >
        <Languages size={15} />
      </button>
    );
  }
  if (variant === "row") {
    return (
      <div
        role="radiogroup"
        aria-label={t.common.language}
        className="flex items-center gap-3 px-2 py-0.5 text-[11.5px] text-ink-3"
      >
        {/* ナビのアイコンと同じ幅（16 px）の枠に置いて、上の行と字の始まりを揃える */}
        <span className="flex w-4 shrink-0 justify-center">
          <Languages size={13} />
        </span>
        <span className="flex items-center gap-2.5">
          {langs.map((l) => (
            // biome-ignore lint/a11y/useSemanticElements: 見た目を揃えたボタン型のラジオ
            <button
              type="button"
              role="radio"
              aria-checked={l === lang}
              key={l}
              onClick={() => setLang(l)}
              className={cn("hover:text-ink", l === lang && "font-medium text-ink")}
            >
              {langNames[l]}
            </button>
          ))}
        </span>
      </div>
    );
  }
  return (
    <Segmented
      size="sm"
      label={t.common.language}
      value={lang}
      onChange={setLang}
      options={langs.map((l) => ({ value: l, label: langNames[l] }))}
    />
  );
}
