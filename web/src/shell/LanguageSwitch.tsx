import { Languages } from "lucide-react";
import { type Lang, langNames, langs, setLang, useLang, useT } from "@/i18n";
import { Segmented } from "@/kit/selection/Segmented";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/ui/dropdown-menu";

/**
 * 表示言語の切り替え（日本語 / English）。選んだ言語は閲覧者ごとに保存する。
 * `row` はサイドバーの下の欄に置く形で、変更履歴の行と同じく「アイコン・名前・いまの値」の 1 行にし、押すとメニューで選ぶ。
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
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button
            type="button"
            className="flex w-full items-center gap-3 rounded-chip px-2 py-0.5 text-[11.5px] text-ink-3 hover:bg-panel"
          >
            {/* ナビのアイコンと同じ幅（16 px）の枠に置いて、上の行と字の始まりを揃える */}
            <span className="flex w-4 shrink-0 justify-center">
              <Languages size={13} />
            </span>
            {t.common.language}
            <span className="ml-auto text-ink-4">{langNames[lang]}</span>
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent side="right" align="end">
          <DropdownMenuRadioGroup
            value={lang}
            onValueChange={(v) => {
              const picked = langs.find((l) => l === v);
              if (picked) setLang(picked);
            }}
          >
            {langs.map((l) => (
              <DropdownMenuRadioItem key={l} value={l}>
                {langNames[l]}
              </DropdownMenuRadioItem>
            ))}
          </DropdownMenuRadioGroup>
        </DropdownMenuContent>
      </DropdownMenu>
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
