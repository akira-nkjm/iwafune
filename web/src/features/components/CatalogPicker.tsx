import { X } from "lucide-react";
import { useState } from "react";
import { useT } from "@/i18n";
import { Command, CommandEmpty, CommandInput, CommandItem, CommandList } from "@/ui/command";
import { filterPickOptions, type PickOption } from "./catalog-pick";

/** 選んだ品の表示（ID は等幅、表示名と補足を添える） */
function Summary({ option }: { option: PickOption }) {
  return (
    <span className="flex min-w-0 flex-1 items-baseline gap-2">
      <span className="shrink-0 font-mono text-[12.5px]">{option.id}</span>
      {option.label && <span className="truncate text-xs">{option.label}</span>}
      {option.note && <span className="truncate text-ink-3 text-xs">{option.note}</span>}
    </span>
  );
}

/**
 * カタログの品を探して選ぶ欄。打つと絞り込み、↑↓ と Enter で選べる。選んだ後はその品を示し、「変更」で選び直す。
 * 一覧に無い ID を打っても選べない（部品が参照している品はもとの一覧に含める）
 */
export function CatalogPicker({
  options,
  value,
  onChange,
}: {
  options: readonly PickOption[];
  value: string;
  onChange: (id: string) => void;
}) {
  const t = useT().components.addPart;
  const [query, setQuery] = useState("");
  const chosen = options.find((o) => o.id === value);
  if (value !== "") {
    return (
      <div
        data-testid="catalog-chosen"
        className="flex items-center gap-2 rounded-[8px] border border-line-strong bg-panel px-2.5 py-1.5"
      >
        <Summary option={chosen ?? { id: value, label: "", note: "" }} />
        <button
          type="button"
          aria-label={t.changeCatalog}
          title={t.changeCatalog}
          className="shrink-0 rounded p-0.5 text-ink-3 hover:text-ink"
          onClick={() => onChange("")}
        >
          <X size={14} />
        </button>
      </div>
    );
  }
  const shown = filterPickOptions(options, query);
  return (
    <Command
      shouldFilter={false}
      label={t.catalog}
      className="h-auto rounded-[8px]! border border-line-strong bg-panel p-0"
    >
      <CommandInput
        aria-label={t.catalog}
        placeholder={t.searchCatalog}
        value={query}
        onValueChange={setQuery}
        // Enter は選ぶだけにして、ダイアログのフォームを送らない
        onKeyDown={(e) => {
          if (e.key === "Enter") e.preventDefault();
        }}
      />
      <CommandList className="max-h-44">
        <CommandEmpty className="py-3 text-ink-3 text-xs">{t.noCatalogMatch}</CommandEmpty>
        {shown.map((o) => (
          <CommandItem key={o.id} value={o.id} onSelect={() => onChange(o.id)}>
            <Summary option={o} />
          </CommandItem>
        ))}
      </CommandList>
    </Command>
  );
}
