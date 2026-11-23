import { Monitor, Moon, Sun } from "lucide-react";
import { useT } from "@/i18n";
import { Segmented } from "@/kit/selection/Segmented";
import { setThemePref, type ThemePref, themePrefs, useThemePref } from "@/lib/theme";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/ui/dropdown-menu";

const ICONS = { system: Monitor, light: Sun, dark: Moon } as const;

/**
 * 画面の配色の切り替え（OS に従う / ライト / ダーク）。選びは閲覧者ごとに保存する（lib/theme.ts）。
 * 形は `LanguageSwitch` と同じ: `row` はサイドバーの下の欄（押すと行のすぐ上にメニューが開く）、`rail` は畳んだサイドバーで
 * 1 つのボタンで順に切り替える、どちらでもなければパネルの見出しなどに置く `Segmented`。
 */
export function ThemeSwitch({ variant = "segmented" }: { variant?: "segmented" | "row" | "rail" }) {
  const pref = useThemePref();
  const c = useT().common;
  const t = {
    appearance: c.appearance,
    themeChoices: { system: c.themeSystem, light: c.themeLight, dark: c.themeDark },
  };
  const Icon = ICONS[pref];
  if (variant === "rail") {
    const next = themePrefs[(themePrefs.indexOf(pref) + 1) % themePrefs.length] ?? "system";
    return (
      <button
        type="button"
        onClick={() => setThemePref(next)}
        title={`${t.appearance}: ${t.themeChoices[pref]} → ${t.themeChoices[next]}`}
        aria-label={`${t.appearance}: ${t.themeChoices[next]}`}
        className="mx-auto flex size-8 items-center justify-center rounded-control text-ink-3 hover:bg-panel hover:text-ink"
      >
        <Icon size={15} />
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
              <Icon size={13} />
            </span>
            {t.appearance}
            <span className="ml-auto text-ink-4">{t.themeChoices[pref]}</span>
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent side="top" align="end" className="w-auto min-w-28">
          <DropdownMenuRadioGroup
            value={pref}
            onValueChange={(v) => {
              const picked = themePrefs.find((p) => p === v);
              if (picked) setThemePref(picked);
            }}
          >
            {themePrefs.map((p) => (
              <DropdownMenuRadioItem key={p} value={p} className="text-xs">
                {t.themeChoices[p]}
              </DropdownMenuRadioItem>
            ))}
          </DropdownMenuRadioGroup>
        </DropdownMenuContent>
      </DropdownMenu>
    );
  }
  return (
    <Segmented<ThemePref>
      size="sm"
      label={t.appearance}
      value={pref}
      onChange={setThemePref}
      options={themePrefs.map((p) => ({ value: p, label: t.themeChoices[p] }))}
    />
  );
}
