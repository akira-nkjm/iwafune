import { QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import {
  isRouteErrorResponse,
  Links,
  Meta,
  Outlet,
  Scripts,
  ScrollRestoration,
} from "react-router";
import { queryClient } from "@/api/queries";
import { getT, useLang } from "@/i18n";
import { SkeletonShell } from "@/kit/feedback/Loading";
import { Toaster } from "@/ui/sonner";
import { TooltipProvider } from "@/ui/tooltip";
import type { Route } from "./+types/root";
import "@/styles/index.css";

export const meta: Route.MetaFunction = () => [{ title: "iwafune" }];

export const links: Route.LinksFunction = () => [
  { rel: "icon", type: "image/svg+xml", href: "/favicon.svg" },
];

/** ドキュメントの殻。SPA モードではビルド時にこれが index.html になる（fune-server が配信する） */
export function Layout({ children }: { children: ReactNode }) {
  const lang = useLang();
  return (
    <html lang={lang}>
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <Meta />
        <Links />
      </head>
      <body>
        <div id="root">{children}</div>
        <ScrollRestoration />
        <Scripts />
      </body>
    </html>
  );
}

export default function App() {
  const lang = useLang();
  return (
    <QueryClientProvider client={queryClient}>
      <TooltipProvider delayDuration={200}>
        {/* 言語を切り替えたら画面を作り直す（getT() で引いた文字も新しい言語になる） */}
        <Outlet key={lang} />
        <Toaster
          position="bottom-right"
          closeButton
          toastOptions={{ classNames: { description: "whitespace-pre-line" } }}
        />
      </TooltipProvider>
    </QueryClientProvider>
  );
}

/**
 * 初回ロード中（SPA の index.html に焼き込まれる）。画面の枠（サイドバーと本体のパネル）の形のスケルトン。
 * 文言は `SkeletonShell` が `useT()` で引く（ハイドレーションの間は焼き込んだ日本語のまま。`getT()` だと英語のときに食い違う）
 */
export function HydrateFallback() {
  return <SkeletonShell />;
}

export function ErrorBoundary({ error }: Route.ErrorBoundaryProps) {
  const t = getT();
  let title = t.shell.errorTitle;
  let detail = t.shell.errorDetail;
  if (isRouteErrorResponse(error)) {
    title = error.status === 404 ? t.shell.pageNotFound : t.shell.statusError(error.status);
    detail = error.statusText || detail;
  } else if (error instanceof Error) {
    detail = error.message;
  }
  return (
    <main className="mx-auto max-w-lg p-8">
      <h1 className="font-semibold text-xl">{title}</h1>
      <p className="mt-2 text-ink-3 text-sm">{detail}</p>
    </main>
  );
}
