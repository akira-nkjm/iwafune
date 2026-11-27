/**
 * ランディングページと docs の入口の文言（英語と日本語）。コマンド・製品の用語は英語のまま。
 * `slug` は docs/ のページ（lib/doc-links.ts の docHref で張る）。
 */
import type { Lang } from "../lib/doc-links.ts";

/** 解析の分野 1 つ（ランディングの「What it analyses」と docs の入口の「Guides by area」）。 */
export interface Area {
  slug: string;
  name: string;
  /** ランディングのカードの説明。 */
  text: string;
  /** docs の入口の一行の説明。 */
  short: string;
}

export interface LandingCopy {
  htmlLang: string;
  title: string;
  description: string;
  skip: string;
  nav: { start: string; guides: string; reference: string; releases: string };
  langSwitch: { label: string; text: string };
  themeToggle: string;
  hero: {
    eyebrow: string;
    title: string;
    lead: string;
    primary: string;
    secondary: string;
    installLabel: string;
    terminalTitle: string;
    terminalAlt: string;
    terminalNote: string;
  };
  principles: { label: string; items: { title: string; text: string }[] };
  app: {
    label: string;
    title: string;
    lead: string;
    windowTitle: string;
    alt: string;
  };
  tour: {
    kicker: string;
    title: string;
    text: string;
    link: string;
    slug: string;
    alt: string;
  }[];
  areas: { title: string; lead: string; items: Area[] };
  start: {
    title: string;
    text: string;
    steps: { before: string; code?: string; after: string }[];
    cta: string;
  };
  footer: { tagline: string; releases: string; source: string };
}

const areasEn: Area[] = [
  {
    slug: "analyses/power",
    name: "Power",
    text: "Mode loads, generation with shadows, energy balance, sizing.",
    short: "energy balance, sizing",
  },
  {
    slug: "analyses/thermal",
    name: "Thermal",
    text: "Steady and transient node temperatures, heaters, couplings.",
    short: "steady, transient",
  },
  {
    slug: "analyses/comm",
    name: "Communication",
    text: "Link budgets, ground-station visibility, data throughput.",
    short: "links, visibility",
  },
  {
    slug: "analyses/aocs",
    name: "AOCS",
    text: "Disturbance torques, wheel unloading, sensor fields of view.",
    short: "torques, unloading",
  },
  {
    slug: "analyses/propulsion",
    name: "Propulsion",
    text: "Delta-v budgets, transfers and swing-bys, propellant per tank.",
    short: "delta-v, transfers",
  },
  {
    slug: "analyses/structure",
    name: "Structure",
    text: "Mass properties, launch fit, whole-body modes, interference.",
    short: "launch fit, modes",
  },
  {
    slug: "analyses/orbit",
    name: "Orbit and radiation",
    text: "Eclipses, decay, trapped-particle flux and total dose.",
    short: "eclipse, dose",
  },
  {
    slug: "analyses/budgets",
    name: "Budgets and compliance",
    text: "Mass, power and data against allocations; requirements table.",
    short: "mass, power, data",
  },
];

const areasJa: Area[] = [
  {
    slug: "analyses/power",
    name: "Power（電力）",
    text: "モードごとの負荷、影を考慮した発電、エネルギー収支、サイジング。",
    short: "エネルギー収支、サイジング",
  },
  {
    slug: "analyses/thermal",
    name: "Thermal（熱）",
    text: "定常・過渡のノード温度、ヒーター、熱結合。",
    short: "定常、過渡",
  },
  {
    slug: "analyses/comm",
    name: "Communication（通信）",
    text: "リンクバジェット、地上局の可視性、データ転送量。",
    short: "リンク、可視性",
  },
  {
    slug: "analyses/aocs",
    name: "AOCS",
    text: "外乱トルク、ホイールのアンローディング、センサの視野。",
    short: "トルク、アンローディング",
  },
  {
    slug: "analyses/propulsion",
    name: "Propulsion（推進）",
    text: "Delta-v バジェット、遷移とスイングバイ、タンクごとの推進剤。",
    short: "delta-v、遷移",
  },
  {
    slug: "analyses/structure",
    name: "Structure（構造）",
    text: "質量特性、打ち上げ時の収まり、全体モード、干渉。",
    short: "収まり、モード",
  },
  {
    slug: "analyses/orbit",
    name: "Orbit and radiation（軌道・放射線）",
    text: "食、軌道減衰、捕捉粒子のフラックスと総線量。",
    short: "食、線量",
  },
  {
    slug: "analyses/budgets",
    name: "Budgets and compliance（バジェット）",
    text: "質量・電力・データを割り当てと比べる。要求の一覧表。",
    short: "質量、電力、データ",
  },
];

/** 分野の並びは英語の名前のアルファベット順（日本語版も同じ順）。 */
const areaOrder = areasEn
  .map((a, index) => ({ name: a.name, index }))
  .sort((a, b) => a.name.localeCompare(b.name, "en"))
  .map((x) => x.index);
const inAreaOrder = (items: Area[]): Area[] =>
  areaOrder.flatMap((index) => {
    const item = items[index];
    return item ? [item] : [];
  });
const sortedAreasEn = inAreaOrder(areasEn);
const sortedAreasJa = inAreaOrder(areasJa);

export const landing: Record<Lang, LandingCopy> = {
  en: {
    htmlLang: "en",
    title: "iwafune — Design a small satellite in plain files",
    description:
      "iwafune is a workbench for the conceptual design of small satellites. Design data lives in a git-friendly directory of TOML; the fune CLI and a web GUI make the same edits and run subsystem analyses.",
    skip: "Skip to content",
    nav: { start: "Get started", guides: "Guides", reference: "Reference", releases: "Releases" },
    langSwitch: { label: "Language", text: "English / 日本語" },
    themeToggle: "Switch between light and dark theme",
    hero: {
      eyebrow: "Satellite design workbench",
      title: "Design a small satellite in plain files.",
      lead: "The design lives in a git-friendly directory of TOML. Edit it from the {fune} CLI or the web GUI, and run subsystem analyses on the same data.",
      primary: "Get started",
      secondary: "Read the concepts →",
      installLabel: "Install the release binary",
      terminalTitle: "my-sat — fune",
      terminalAlt:
        "A terminal session: create a project, place a part, set a value, check, analyse, open the GUI",
      terminalNote:
        "Every GUI edit shows the same command, so a session is easy to repeat or review.",
    },
    principles: {
      label: "Principles",
      items: [
        {
          title: "Design data in git",
          text: "Parts, modes, cases and requirements are TOML files with units and sources. Diffs read like design changes.",
        },
        {
          title: "CLI and GUI as equals",
          text: "Both make the same edits through the same operations and write byte-identical files.",
        },
        {
          title: "Analyses with verdicts",
          text: "Each analysis reports pass, warning or violation, and lists the inputs that are still missing.",
        },
      ],
    },
    app: {
      label: "The app",
      title: "The same design, in the browser",
      lead: "{fune gui} opens the directory you are in. Margins for mass, power, storage, link and delta-v on one page, each linked to the analysis behind it.",
      windowTitle: "fune gui — IWAFUNE 6U (demo)",
      alt: "The overview page of the demo satellite: margins for mass, power, storage, link and delta-v, and the orbit conditions",
    },
    tour: [
      {
        kicker: "01 — Parts and budgets",
        title: "Every value has a unit and a source.",
        text: "Pick parts from a catalog, record where each number comes from, and track mass, power and data against allocations and margins.",
        link: "Budgets guide →",
        slug: "analyses/budgets",
        alt: "The components page: the parts tree and a part's attributes with their sources",
      },
      {
        kicker: "02 — Layout",
        title: "Place parts in 3D, and let the geometry count.",
        text: "Shadows on cells, drag area, covered faces and interference all come from the same boxes you place.",
        link: "Layout guide →",
        slug: "guide/gui",
        alt: "The layout page: the parts tree beside an editable 3D view with a view cube",
      },
      {
        kicker: "03 — Analyses",
        title: "Verdicts, and what is still missing.",
        text: "Power, thermal, communication, attitude control, propulsion, structure, orbit and radiation. A missing input can be fixed from the row that reports it.",
        link: "Analyses reference →",
        slug: "analyses/index",
        alt: "The thermal page: a 3D thermal image and the node temperatures for the worst case",
      },
    ],
    areas: {
      title: "What it analyses",
      lead: "One guide per area, each starting from the demo satellite.",
      items: sortedAreasEn,
    },
    start: {
      title: "Ten minutes with the demo",
      text: "Open the fictional 6U demo, change one value from the GUI, and watch the budgets and verdicts follow.",
      steps: [
        { before: "Install ", code: "fune", after: "" },
        { before: "Open the demo with ", code: "fune gui", after: "" },
        { before: "Edit a part and read the command it made", after: "" },
        { before: "Run the analyses and fix a missing input", after: "" },
      ],
      cta: "Open the quickstart →",
    },
    footer: {
      tagline: "iwafune — a workbench for the conceptual design of small satellites",
      releases: "Release notes",
      source: "Source on GitHub",
    },
  },
  ja: {
    htmlLang: "ja",
    title: "iwafune — 小型衛星の設計を、ふつうのファイルで",
    description:
      "iwafune は小型衛星の概念設計のための作業台です。設計データは git で扱える TOML のディレクトリにあり、fune CLI と Web GUI のどちらからも同じ編集と各サブシステムの解析ができます。",
    skip: "本文へ移動",
    nav: { start: "はじめる", guides: "ガイド", reference: "リファレンス", releases: "リリース" },
    langSwitch: { label: "言語", text: "English / 日本語" },
    themeToggle: "明るい配色と暗い配色を切り替える",
    hero: {
      eyebrow: "Satellite design workbench",
      title: "小型衛星の設計を、ふつうのファイルで。",
      lead: "設計データは git で扱いやすい TOML のディレクトリにあります。{fune} CLI でも Web GUI でも編集でき、同じデータに対して各サブシステムの解析を実行できます。",
      primary: "はじめる",
      secondary: "基本の考え方を読む →",
      installLabel: "リリースのバイナリをインストール",
      terminalTitle: "my-sat — fune",
      terminalAlt:
        "端末の操作例: プロジェクトを作り、部品を置き、値を設定し、検査と解析を実行して GUI を開く",
      terminalNote:
        "GUI での編集には同じコマンドが表示されるので、操作を繰り返したり見直したりしやすくなります。",
    },
    principles: {
      label: "考え方",
      items: [
        {
          title: "設計データは git に",
          text: "部品、モード、ケース、要求は、単位と出典をもつ TOML ファイルです。差分がそのまま設計変更として読めます。",
        },
        {
          title: "CLI と GUI は対等",
          text: "どちらも同じ操作で同じ編集を行い、バイト単位で同じファイルを書きます。",
        },
        {
          title: "判定つきの解析",
          text: "各解析は合格・警告・違反を報告し、まだ足りない入力も一覧にします。",
        },
      ],
    },
    app: {
      label: "アプリ",
      title: "同じ設計を、ブラウザで",
      lead: "{fune gui} は、いまいるディレクトリを開きます。質量・電力・記録容量・通信リンク・delta-v の余裕を 1 ページに並べ、それぞれの根拠になる解析へたどれます。",
      windowTitle: "fune gui — IWAFUNE 6U (demo)",
      alt: "デモ衛星の概要ページ。質量、電力、記録容量、通信リンク、delta-v の余裕と、軌道条件",
    },
    tour: [
      {
        kicker: "01 — 部品とバジェット",
        title: "すべての値に、単位と出典がある。",
        text: "カタログから部品を選び、各数値の出どころを記録して、質量・電力・データを割り当てと余裕に照らして管理します。",
        link: "バジェットの解説 →",
        slug: "analyses/budgets",
        alt: "コンポーネントのページ。部品のツリーと、部品の属性およびその出典",
      },
      {
        kicker: "02 — 配置",
        title: "部品を 3D で置けば、形状がそのまま計算に効く。",
        text: "セルへの影、空気抵抗の受圧面積、覆われた面、干渉は、置いたボックスから求めます。",
        link: "配置の解説 →",
        slug: "guide/gui",
        alt: "配置のページ。部品のツリーと、ビューキューブつきの編集できる 3D ビュー",
      },
      {
        kicker: "03 — 解析",
        title: "判定と、まだ足りないもの。",
        text: "電力、熱、通信、姿勢制御、推進、構造、軌道、放射線。足りない入力は、それを報告した行から直せます。",
        link: "解析のリファレンス →",
        slug: "analyses/index",
        alt: "熱のページ。3D の温度分布と、最悪ケースのノード温度",
      },
    ],
    areas: {
      title: "解析できること",
      lead: "分野ごとに 1 つのガイドがあり、どれもデモ衛星から始まります。",
      items: sortedAreasJa,
    },
    start: {
      title: "デモで 10 分",
      text: "架空の 6U デモを開き、GUI から値を 1 つ変えて、バジェットと判定が追従するのを確かめます。",
      steps: [
        { before: "", code: "fune", after: " をインストールする" },
        { before: "", code: "fune gui", after: " でデモを開く" },
        { before: "部品を編集して、生成されたコマンドを読む", after: "" },
        { before: "解析を実行して、足りない入力を直す", after: "" },
      ],
      cta: "クイックスタートを開く →",
    },
    footer: {
      tagline: "iwafune — 小型衛星の概念設計のための作業台",
      releases: "リリースノート",
      source: "GitHub のソース",
    },
  },
};

/** docs の入口（option C）の文言。 */
export interface DocsEntryCopy {
  title: string;
  description: string;
  lead: string;
  primary: string;
  secondary: string;
  startHere: string;
  cards: { kicker: string; title: string; text: string; slug: string }[];
  guides: string;
  layout: { name: string; short: string; slug: string };
  install: { title: string; text: string; more: string };
  release: { title: string; version: string; notes: string };
  areas: Area[];
}

export const docsEntry: Record<Lang, DocsEntryCopy> = {
  en: {
    title: "iwafune documentation",
    description:
      "Documentation for iwafune: install it, take the ten-minute tour of the demo, learn the concepts, and read one guide per analysis area.",
    lead: "A workbench for the conceptual design of small satellites. Design data in git, a CLI and a GUI that make the same edits, and analyses for each subsystem.",
    primary: "Quickstart",
    secondary: "Install",
    startHere: "Start here",
    cards: [
      {
        kicker: "10 min",
        title: "Quickstart with the demo",
        text: "Open the 6U demo, edit a part in the GUI, run the analyses.",
        slug: "guide/quickstart",
      },
      {
        kicker: "Concepts",
        title: "How the design data works",
        text: "Parts, modes, cases and configurations; units and sources.",
        slug: "guide/concepts",
      },
      {
        kicker: "Tutorial",
        title: "Start your own satellite",
        text: "From fune init to a first mass and power budget.",
        slug: "guide/first-satellite",
      },
      {
        kicker: "Reference",
        title: "CLI and analyses",
        text: "Every command, and one page per analysis area.",
        slug: "guide/cli",
      },
    ],
    guides: "Guides by area",
    layout: { name: "3D layout", short: "placing parts", slug: "guide/gui" },
    install: {
      title: "Install",
      text: "Linux (glibc 2.39 or later) and macOS. The binary has the GUI embedded.",
      more: "Other ways to install →",
    },
    release: {
      title: "Latest release",
      version: "Version {version}",
      notes: "Release notes and binaries on GitHub",
    },
    areas: sortedAreasEn,
  },
  ja: {
    title: "iwafune ドキュメント",
    description:
      "iwafune のドキュメント。インストール、デモの 10 分ツアー、基本の考え方、解析の分野ごとのガイド。",
    lead: "小型衛星の概念設計のための作業台です。git で扱う設計データ、同じ編集をする CLI と GUI、各サブシステムの解析を備えています。",
    primary: "クイックスタート",
    secondary: "インストール",
    startHere: "はじめに",
    cards: [
      {
        kicker: "10 分",
        title: "デモで試すクイックスタート",
        text: "6U のデモを開き、GUI で部品を編集して、解析を実行します。",
        slug: "guide/quickstart",
      },
      {
        kicker: "基本",
        title: "設計データのしくみ",
        text: "部品、モード、ケース、構成。単位と出典。",
        slug: "guide/concepts",
      },
      {
        kicker: "チュートリアル",
        title: "自分の衛星を始める",
        text: "fune init から、最初の質量・電力バジェットまで。",
        slug: "guide/first-satellite",
      },
      {
        kicker: "リファレンス",
        title: "CLI と解析",
        text: "すべてのコマンドと、解析の分野ごとのページ。",
        slug: "guide/cli",
      },
    ],
    guides: "分野別のガイド",
    layout: { name: "3D 配置", short: "部品を置く", slug: "guide/gui" },
    install: {
      title: "インストール",
      text: "Linux（glibc 2.39 以降）と macOS。バイナリに GUI が組み込まれています。",
      more: "ほかのインストール方法 →",
    },
    release: {
      title: "最新のリリース",
      version: "バージョン {version}",
      notes: "GitHub のリリースノートとバイナリ",
    },
    // 入口の一覧は幅が狭いので、分野の名前は英語のままにする。
    areas: sortedAreasJa.map((a, i) => ({ ...a, name: sortedAreasEn[i]?.name ?? a.name })),
  },
};

/** インストールの 1 行（README と同じ）。 */
export const INSTALL_COMMAND =
  "cargo binstall --git https://github.com/akira-nkjm/iwafune fune-cli";
