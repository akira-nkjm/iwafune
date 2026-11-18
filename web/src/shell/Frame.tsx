import { useQuery } from "@tanstack/react-query";
import { type ReactNode, useEffect } from "react";
import { useNavigation } from "react-router";
import { q } from "@/api/queries";
import { cn } from "@/lib/cn";
import { PreloadContext, type PreloadPlan, preloadIdlePages } from "./preload";
import { Sidebar } from "./sidebar/Sidebar";
import { SidebarProvider } from "./sidebar/SidebarContext";
import { useHistoryKeys } from "./use-history-keys";

/**
 * 画面の枠: 232px の固定サイドバー + 10px 内側に浮いた白いメインパネル 1 枚。
 * `preload` は先読みの計画（ページの登録 `routes/pages.ts` が作る）。ナビの項目は `usePreloadPlan` で引く
 */
export function Frame({ children, preload }: { children: ReactNode; preload: PreloadPlan }) {
  const navigation = useNavigation();
  // ⌘Z・⇧⌘Z（Ctrl+Z・Ctrl+Y）で直前の編集を元に戻す・やり直す（どのページでも。欄に文字を打っている間は欄の取り消し）
  useHistoryKeys();
  // 編集のトーストに「質量 10.84 → 10.86 kg」を出すため、バジェットを先に読んでおく（api/ops.ts が前後を比べる）
  useQuery(q.budget("mass"));
  useQuery(q.budget("power"));
  useQuery(q.budget("data"));
  // 解析の一覧（系のページはここから解析を引く）。先に読んでおき、系のページを開いたらすぐ計算を頼めるようにする
  useQuery(q.analyses());
  // 軽いページの本体は空き時間に読んでおく（移った瞬間に中身を描けるように）
  useEffect(() => preloadIdlePages(preload.idle), [preload]);
  return (
    <PreloadContext value={preload}>
      <SidebarProvider>
        <div className="flex h-dvh min-h-0">
          <div
            aria-hidden="true"
            className={cn(
              "fixed inset-x-0 top-0 z-50 h-0.5 origin-left bg-accent transition-transform duration-500",
              navigation.state === "loading" ? "scale-x-75" : "scale-x-0 duration-0",
            )}
          />
          <Sidebar />
          <main className="flex min-h-0 min-w-0 flex-1 flex-col py-2.5 pr-2.5">
            <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-panel border border-line bg-panel shadow-panel">
              {children}
            </div>
          </main>
        </div>
      </SidebarProvider>
    </PreloadContext>
  );
}
