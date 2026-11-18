import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { expect, it } from "vitest";
import type { ModeHeader, QuantityView } from "@/api/types";
import { getT } from "@/i18n";
import { ModeDuration } from "./ModeDuration";

const mode = (max_duration: QuantityView | null): ModeHeader => ({
  id: "DOWNLINK",
  label: "",
  description: null,
  file: "modes/DOWNLINK.toml",
  max_duration,
  when: null,
  start: null,
});

const render = (m: ModeHeader) =>
  renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>
      <ModeDuration mode={m} />
    </QueryClientProvider>,
  );

it("最長継続時間はふだん入力欄でなく小さな文字で出し、未設定なら追加の案内を出す", () => {
  const t = getT().modes;
  const set = render(mode({ display: "15.0 min" } as QuantityView));
  expect(set).toContain(t.durationAtMost("15.0 min"));
  expect(set).not.toContain("<input");
  const unset = render(mode(null));
  expect(unset).toContain(t.durationAdd);
  expect(unset).toContain(`aria-label="${t.maxDuration}: ${t.durationUnset}"`);
});
