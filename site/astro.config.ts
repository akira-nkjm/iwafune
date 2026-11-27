import { unified } from "@astrojs/markdown-remark";
import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";
import rehypeKatex from "rehype-katex";
import remarkMath from "remark-math";
import docsFiles from "./src/integrations/docs-files.ts";
import { buildSidebar } from "./src/lib/sidebar.ts";
import { BASE, docsDir, REPO_URL, repoRoot, SITE_ORIGIN } from "./src/lib/site.ts";
import rehypeTableWrap from "./src/plugins/rehype-table-wrap.ts";
import remarkDocsImages from "./src/plugins/remark-docs-images.ts";
import remarkDocsLinks from "./src/plugins/remark-docs-links.ts";
import remarkDocsTitle from "./src/plugins/remark-docs-title.ts";
import remarkFences from "./src/plugins/remark-fences.ts";
import remarkGithubAlerts from "./src/plugins/remark-github-alerts.ts";
import remarkScreenshots from "./src/plugins/remark-screenshots.ts";

const docs = docsDir();

export default defineConfig({
  site: SITE_ORIGIN,
  base: BASE,
  trailingSlash: "always",
  server: { host: "localhost", port: 4321 },
  vite: {
    // docs/ はこのプロジェクトの外にあるので、読むことを許す。
    server: { fs: { allow: [repoRoot(docs), "."] } },
  },
  markdown: {
    processor: unified({
      remarkPlugins: [
        remarkMath,
        remarkFences,
        remarkGithubAlerts,
        remarkDocsTitle,
        remarkDocsLinks,
        remarkDocsImages,
        remarkScreenshots,
      ],
      rehypePlugins: [[rehypeKatex, { throwOnError: false }], rehypeTableWrap],
    }),
  },
  integrations: [
    docsFiles(),
    starlight({
      title: { en: "iwafune", ja: "iwafune" },
      description:
        "A workbench for the conceptual design of small satellites: design data in git, a CLI and a GUI that make the same edits, and analyses for each subsystem.",
      logo: {
        light: "../docs/assets/logo-light.svg",
        dark: "../docs/assets/logo-dark.svg",
        alt: "iwafune",
        replacesTitle: true,
      },
      favicon: "/favicon.svg",
      defaultLocale: "root",
      locales: {
        root: { label: "English", lang: "en" },
        ja: { label: "日本語", lang: "ja" },
      },
      social: [{ icon: "github", label: "GitHub", href: REPO_URL }],
      sidebar: buildSidebar(docs),
      tableOfContents: { minHeadingLevel: 2, maxHeadingLevel: 3 },
      lastUpdated: false,
      customCss: [
        "@fontsource-variable/noto-sans-jp/index.css",
        "@fontsource-variable/noto-sans-mono/index.css",
        "katex/dist/katex.min.css",
        "./src/styles/tokens.css",
        "./src/styles/starlight-theme.css",
        "./src/styles/prose.css",
      ],
      components: {
        Head: "./src/components/Head.astro",
      },
      expressiveCode: {
        themes: ["github-dark", "github-light"],
        styleOverrides: {
          codeBackground: "var(--iw-code-bg)",
          borderColor: "var(--iw-line-strong)",
        },
      },
    }),
  ],
});
