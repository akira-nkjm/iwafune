import { Languages } from "lucide-react";
import { type Lang, langNames, langs, setLang, useLang, useT } from "@/i18n";
import { Segmented } from "@/kit/selection/Segmented";

/** 表示言語の切り替え（日本語 / English）。選んだ言語は閲覧者ごとに保存する。rail では 1 つのボタンで交互に切り替える */
export function LanguageSwitch({ rail = false }: { rail?: boolean }) {
  const lang = useLang();
  const t = useT();
  if (rail) {
    const next: Lang = lang === "ja" ? "en" : "ja";
    return (
      <button
        type="button"
        onClick={() => setLang(next)}
        title={`${t.common.language}: ${langNames[lang]} → ${langNames[next]}`}
        aria-label={`${t.common.language}: ${langNames[next]}`}
        className="mx-auto flex size-9 items-center justify-center rounded-control text-ink-3 hover:bg-panel hover:text-ink"
      >
        <Languages size={15} />
      </button>
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
