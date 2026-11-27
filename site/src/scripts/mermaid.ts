/**
 * ```mermaid の図をブラウザで描く。
 * 図のあるページ（`pre.mermaid`）だけが Mermaid を読み込む。配色は CSS の色の変数（styles/tokens.css）から取り、
 * 明るい／暗いの切り替え（<html data-theme>）に合わせて描き直す。
 */

const SELECTOR = "pre.mermaid";

/** CSS の変数の値を読む。 */
function token(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(`--iw-${name}`).trim();
}

/** いまの配色に合わせた Mermaid の設定。 */
function themeVariables(): Record<string, string> {
  return {
    background: token("panel"),
    fontFamily: token("font-text") || "sans-serif",
    fontSize: "15px",
    primaryColor: token("accent-tint"),
    primaryTextColor: token("ink"),
    primaryBorderColor: token("accent"),
    secondaryColor: token("panel-2"),
    secondaryTextColor: token("ink"),
    secondaryBorderColor: token("line-strong"),
    tertiaryColor: token("panel"),
    tertiaryTextColor: token("ink"),
    tertiaryBorderColor: token("line-strong"),
    lineColor: token("ink-3"),
    textColor: token("ink"),
    nodeBorder: token("accent"),
    mainBkg: token("accent-tint"),
    clusterBkg: token("panel-2"),
    clusterBorder: token("line-strong"),
    edgeLabelBackground: token("panel"),
    noteBkgColor: token("warn-bg"),
    noteTextColor: token("ink"),
    noteBorderColor: token("warn"),
    actorBkg: token("accent-tint"),
    actorBorder: token("accent"),
    actorTextColor: token("ink"),
    signalColor: token("ink-3"),
    signalTextColor: token("ink"),
  };
}

async function start(nodes: HTMLElement[]): Promise<void> {
  const { default: mermaid } = await import("mermaid");
  const sources = new Map(nodes.map((n) => [n, n.textContent ?? ""]));
  let running = Promise.resolve();

  const render = () => {
    running = running.then(async () => {
      mermaid.initialize({
        startOnLoad: false,
        securityLevel: "strict",
        theme: "base",
        themeVariables: themeVariables(),
      });
      for (const [node, source] of sources) {
        node.removeAttribute("data-processed");
        node.textContent = source;
      }
      await mermaid.run({ nodes: [...sources.keys()], suppressErrors: true });
    });
  };

  render();
  new MutationObserver(render).observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme"],
  });
}

const nodes = [...document.querySelectorAll<HTMLElement>(SELECTOR)];
if (nodes.length > 0) void start(nodes);
