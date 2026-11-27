/**
 * 写真の準備に使うコマンドの読み取り: 1 行のコマンドを語に分ける関数と、チュートリアルの文書が載せているコマンドを順に取り出す関数。
 */

/** コマンド行を語に分ける（空白区切り、二重引用符と単一引用符で 1 語にまとめる、`\` は引用符の外では次の 1 文字を残す）。 */
export function tokenize(line: string): string[] {
  const out: string[] = [];
  let cur = "";
  let started = false;
  let quote: '"' | "'" | null = null;
  for (let i = 0; i < line.length; i++) {
    const c = line[i] as string;
    if (quote) {
      if (c === quote) quote = null;
      else if (c === "\\" && quote === '"' && i + 1 < line.length) cur += line[++i];
      else cur += c;
    } else if (c === '"' || c === "'") {
      quote = c;
      started = true;
    } else if (/\s/.test(c)) {
      if (started) out.push(cur);
      cur = "";
      started = false;
    } else if (c === "\\" && i + 1 < line.length) {
      cur += line[++i];
      started = true;
    } else {
      cur += c;
      started = true;
    }
  }
  if (quote) throw new Error(`unterminated quote in: ${line}`);
  if (started) out.push(cur);
  return out;
}

/**
 * 文書の中の ```console ブロックから、`$ ` で始まるコマンドを順に取り出す（行末の `\` でつながる行は 1 つにする）。
 * `stopAt` が文字列なら、その最初の出現より前だけを読む（そこまでの手順を再現するため）。
 */
export function consoleCommands(markdown: string, stopAt?: string): string[] {
  const end = stopAt ? markdown.indexOf(stopAt) : -1;
  if (stopAt && end < 0) throw new Error(`marker not found in the page: ${stopAt}`);
  const text = end >= 0 ? markdown.slice(0, end) : markdown;
  const out: string[] = [];
  for (const block of text.matchAll(/^```console\n([\s\S]*?)^```/gm)) {
    let pending: string | null = null;
    for (const line of (block[1] ?? "").split("\n")) {
      if (pending !== null) {
        pending += ` ${line.trim()}`;
      } else if (line.startsWith("$ ")) {
        pending = line.slice(2);
      } else {
        continue;
      }
      if (pending.endsWith("\\")) {
        pending = pending.slice(0, -1).trimEnd();
        continue;
      }
      out.push(pending.trim());
      pending = null;
    }
  }
  return out;
}
