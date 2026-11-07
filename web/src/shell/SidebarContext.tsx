import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  isToggleShortcut,
  readClosedSections,
  readOpen,
  writeClosedSections,
  writeOpen,
} from "./sidebar-state";

interface SidebarState {
  open: boolean;
  toggle: () => void;
  closedSections: ReadonlySet<string>;
  toggleSection: (id: string) => void;
}

const Ctx = createContext<SidebarState>({
  open: true,
  toggle: () => {},
  closedSections: new Set(),
  toggleSection: () => {},
});

/** サイドバーの開閉（⌘/Ctrl+B）と区分の開閉。状態は閲覧者ごとに保存する */
export function SidebarProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(true);
  const [closed, setClosed] = useState<ReadonlySet<string>>(new Set());
  // 保存した状態はハイドレーションの後に読む（index.html は開いた状態で焼かれる）
  useEffect(() => {
    setOpen(readOpen());
    setClosed(new Set(readClosedSections()));
  }, []);
  const toggle = useCallback(() => {
    setOpen((o) => {
      writeOpen(!o);
      return !o;
    });
  }, []);
  const toggleSection = useCallback((id: string) => {
    setClosed((c) => {
      const next = new Set(c);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      writeClosedSections([...next]);
      return next;
    });
  }, []);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!isToggleShortcut(e)) return;
      e.preventDefault();
      toggle();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [toggle]);
  const value = useMemo(
    () => ({ open, toggle, closedSections: closed, toggleSection }),
    [open, toggle, closed, toggleSection],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useSidebar(): SidebarState {
  return useContext(Ctx);
}
