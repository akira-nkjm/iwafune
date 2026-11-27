/**
 * 写真ごとの撮り方（シナリオ）。キーは写真の識別子（`<slug>` か、名前付きの目印の `<slug>--<name>`。規則は src/lib/shots.ts）。
 * シナリオが無い写真は、デモのルートをそのまま開いて、画面が落ち着くのを待って撮る。
 * 操作は英語・日本語のどちらの画面でも動くように、文言ではなく構造（行・見出し・canvas）で場所を選ぶ。
 */
import type { Locator, Page } from "playwright-core";
import type { ProjectSpec } from "./project.ts";
import { waitForCanvas } from "./wait.ts";

/** 操作に渡すもの。 */
export interface StepContext {
  page: Page;
  /** 写真の名前（誤りの文面に使う）。 */
  label: string;
  /** 画面がもう一度落ち着くまで待つ。 */
  settle(): Promise<void>;
}

/** 写真 1 枚の撮り方。 */
export interface Scenario {
  /** 開くプロジェクト（省略するとデモ）。 */
  project?: ProjectSpec;
  /** ルートを開いて落ち着いたあとの操作（行を開く、要素を見える所へ動かす、描画を待つ、など）。 */
  steps?: (ctx: StepContext) => Promise<void>;
  /** 画面の高さ（CSS ピクセル）。縦に長いページを 1 枚に収めるときだけ、既定の 900 より大きくする。 */
  height?: number;
  /** 撮る範囲を要素に絞る（省略すると画面全体）。 */
  clip?: (page: Page) => Locator;
}

/** デモ（何も変えない）。 */
export const DEMO: ProjectSpec = { kind: "demo" };

/** 「Missing inputs」の写真の元になる最小のプロジェクト（電池だけがあり、容量などが無い）。 */
const MISSING_INPUTS_PROJECT: ProjectSpec = {
  kind: "init",
  cli: [
    "fune source set judgement_1 --kind judgement --label Estimate --date 2026-11-01",
    'fune catalog add local/bat_pack --kind battery --label "Battery pack" --maturity off_the_shelf',
    'fune catalog set local/bat_pack mass "250 g" --source judgement_1',
    "fune part add BAT --catalog local/bat_pack --group power",
  ],
};

/** Overview の質量バジェットのパネルを画面の上端へ動かす（予算のパネルと、その下の診断の見出しが見える）。 */
async function scrollToBudgets({ page, settle }: StepContext): Promise<void> {
  await page
    .locator("section:has(h2)")
    .nth(1)
    .evaluate((el) => el.scrollIntoView({ block: "start" }));
  await settle();
}

export const SCENARIOS: Record<string, Scenario> = {
  // Overview 全体（KPI のカードから予算のパネルと診断まで）。縦に長いので、画面を高くして 1 枚に収める。
  "home--budgets": { height: 1680 },
  // Overview: 質量バジェットのパネルで Payload の行が配分を超えた状態。
  "home--mass-over": {
    project: {
      kind: "demo",
      cli: ['fune set CAM.mass "1.2 kg" --source em_test_2026_10 --note "Heavier lens barrel"'],
    },
    steps: scrollToBudgets,
  },
  // 解析の行を開いて、足りない入力とその直し方（Enter value、Add a part）を出す。
  "systems-power--missing-inputs": {
    project: MISSING_INPUTS_PROJECT,
    steps: async ({ page, settle }) => {
      // 解析の ID（power.sizing）で行を選ぶ。ID は翻訳されない。
      const row = page.locator("tbody tr", { hasText: "power.sizing" }).first();
      await row.click();
      await row.scrollIntoViewIfNeeded();
      await settle();
    },
  },
  // Layout: 3D のビュー（ビューキューブ、回転の矢印、ズームなどのボタン）。
  "design-layout--view-controls": {
    steps: async ({ page, label, settle }) => {
      await waitForCanvas(page, label);
      await settle();
    },
  },
  // チュートリアルで作る小さなプロジェクトの Layout。
  "design-layout--first-satellite": {
    project: { kind: "tutorial", stopAt: "<!-- screenshot: /design/layout" },
    steps: async ({ page, label, settle }) => {
      await waitForCanvas(page, label);
      await settle();
    },
  },
  // 部品の詳細: 属性の表から下の Placement のセクションまでを見せる（詳細のパネルを Placement の所まで動かす）。
  "design-components-cam": {
    steps: async ({ page, settle }) => {
      const heading = page.getByRole("heading", { name: /^(Placement|配置)$/ });
      await heading.evaluate((el) => el.scrollIntoView({ block: "center" }));
      await settle();
    },
  },
  // 熱画像: 温度で色分けした 3D。
  "systems-thermal-heat": {
    steps: async ({ page, label, settle }) => {
      await waitForCanvas(page, label);
      await settle();
    },
  },
};
