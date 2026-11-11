import { Search } from "lucide-react";
import { cn } from "@/lib/cn";
import { TextInput } from "./TextInput";

/**
 * 虫眼鏡のアイコン付きの検索欄（一覧のページの見出し）。打つたびに `onChange(文字列)`。
 * 絞り込みの規則は `search.ts` の `matchesSearch`。`placeholder` を省くと `label` を出す
 */
export function SearchInput({
  value,
  onChange,
  label,
  placeholder = label,
  className,
}: {
  value: string;
  onChange: (value: string) => void;
  label: string;
  placeholder?: string;
  className?: string;
}) {
  return (
    <div className={cn("relative w-52", className)}>
      <Search size={13} aria-hidden className="absolute top-2.5 left-2.5 text-ink-4" />
      <TextInput
        aria-label={label}
        placeholder={placeholder}
        className="pl-7"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
    </div>
  );
}
