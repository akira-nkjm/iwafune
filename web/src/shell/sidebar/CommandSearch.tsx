import { Search } from "lucide-react";
import { useEffect, useState } from "react";
import { useNavigate } from "react-router";
import { useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { nodeName } from "@/lib/design";
import { nav, navItemTitle, paths } from "@/lib/nav";
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/ui/command";
import { Kbd } from "@/ui/kbd";
import { RailTip } from "./RailTip";
import { railFade } from "./sidebar-motion";

/** ⌘K / Ctrl+K: ページ・部品・ネット・モードへ飛ぶ */
export function CommandSearch({ rail = false }: { rail?: boolean }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const navigate = useNavigate();
  const design = useDesign();
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key.toLowerCase() === "k" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setOpen((o) => !o);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  const go = (to: string) => {
    setOpen(false);
    navigate(to);
  };
  const d = design.data;
  return (
    <>
      {/* 開いても畳んでも同じ形（アイコンの位置が動かない）。名前とキーだけを薄く消す */}
      <RailTip rail={rail} label={t.shell.searchKey}>
        <button
          type="button"
          onClick={() => setOpen(true)}
          aria-label={rail ? t.shell.searchKey : undefined}
          className="mb-3 flex h-8 w-full items-center gap-2 overflow-hidden rounded-control border border-line bg-panel px-[7px] text-[12.5px] text-ink-3 hover:text-ink"
        >
          <Search size={16} className="shrink-0" />
          <span className={railFade(rail)}>{t.shell.search}</span>
          <Kbd className={cn("ml-auto text-2xs", railFade(rail))}>⌘K</Kbd>
        </button>
      </RailTip>
      <CommandDialog
        open={open}
        onOpenChange={setOpen}
        title={t.shell.searchTitle}
        description={t.shell.searchDescription}
      >
        <CommandInput placeholder={t.shell.searchPlaceholder} />
        <CommandList>
          <CommandEmpty>{t.shell.notFound}</CommandEmpty>
          <CommandGroup heading={t.shell.groupPages}>
            {nav(d?.defs.systems ?? []).flatMap((s) =>
              s.items.map((it) => (
                <CommandItem
                  key={it.to}
                  value={`page ${navItemTitle(it, t, d?.defs)} ${it.key} ${it.to}`}
                  onSelect={() => go(it.to)}
                >
                  <it.icon size={14} />
                  <span className="text-box-cap">
                    {s.heading ? `${t.nav.sections[s.id]} / ` : ""}
                    {navItemTitle(it, t, d?.defs)}
                  </span>
                </CommandItem>
              )),
            )}
          </CommandGroup>
          {d && (
            <CommandGroup heading={t.shell.groupParts}>
              {d.parts.map((p) => (
                <CommandItem
                  key={p.id}
                  value={`part ${p.id} ${p.kind ?? ""} ${p.catalog ?? ""}`}
                  onSelect={() => go(paths.component(p.id))}
                >
                  <span className="font-mono">{p.id}</span>
                  <span className="text-ink-3 text-xs">{p.kind}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          )}
          {d && d.nets.length > 0 && (
            <CommandGroup heading={t.shell.groupNets}>
              {d.nets.map((n) => (
                <CommandItem
                  key={n.id}
                  value={`net ${n.id} ${n.kind}`}
                  onSelect={() => go(paths.diagram())}
                >
                  <span className="font-mono">{n.id}</span>
                  <span className="text-ink-3 text-xs">{n.kind}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          )}
          {d && d.modes.modes.length > 0 && (
            <CommandGroup heading={t.shell.groupModes}>
              {d.modes.modes.map((m) => (
                <CommandItem
                  key={m.id}
                  value={`mode ${m.id} ${m.label}`}
                  onSelect={() => go(paths.modes())}
                >
                  <span className="font-mono">{m.id}</span>
                  {nodeName(m) !== m.id && (
                    <span className="text-ink-3 text-xs">{nodeName(m)}</span>
                  )}
                </CommandItem>
              ))}
            </CommandGroup>
          )}
        </CommandList>
      </CommandDialog>
    </>
  );
}
