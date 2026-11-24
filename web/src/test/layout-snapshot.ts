import type { DiagramLayout } from "@/features/diagram/layout/geometry";

/** DiagramLayout を比べられる形（Map を、キー順に並べた配列）にする。位置・経路・ラベル・端子・分かれ口を全部含む */
export function snapshotOf(l: DiagramLayout): unknown {
  const sorted = <V>(m: ReadonlyMap<string, V>) =>
    [...m].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
  return {
    boxes: sorted(l.boxes),
    absolute: sorted(l.absolute),
    ports: sorted(l.ports),
    taps: sorted(l.taps),
    routes: sorted(l.routes),
    labels: sorted(l.labels),
    unrouted: [...(l.unrouted ?? [])].sort(),
  };
}

/** 文字列の SHA-256（16 進） */
export async function sha256(text: string): Promise<string> {
  const bytes = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  return [...new Uint8Array(bytes)].map((b) => b.toString(16).padStart(2, "0")).join("");
}
