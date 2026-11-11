import { useState } from "react";

/**
 * いくつかの編集ダイアログのうち、開いている 1 つ（`current`。無ければ null）。ダイアログは開いている間だけ描き、
 * `props`（`open` と、閉じたら `current` を戻す `onOpenChange`）を渡す:
 *
 * ```tsx
 * const dialogs = useDialogs<"rename" | "remove">();
 * <DetailAction onClick={() => dialogs.show("rename")} … />
 * {dialogs.current === "rename" && <RenameDialog {...dialogs.props} … />}
 * ```
 */
export function useDialogs<K extends string>() {
  const [current, setCurrent] = useState<K | null>(null);
  return {
    current,
    show: (kind: K) => setCurrent(kind),
    props: {
      open: true,
      onOpenChange: (open: boolean) => {
        if (!open) setCurrent(null);
      },
    },
  };
}
