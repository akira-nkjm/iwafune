import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { expect, it } from "vitest";
import type { MissingInput } from "@/api/types";
import { getT } from "@/i18n";
import { MissingList } from "./MissingList";

const render = (node: ReactNode) =>
  renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>
      <MemoryRouter>{node}</MemoryRouter>
    </QueryClientProvider>,
  );

const withFixes = (fixes: MissingInput["fixes"]): MissingInput => ({
  part: "RW_X",
  path: "RW_X.max_momentum",
  target: { kind: "param", target: "attr", part: "RW_X", attr: "max_momentum", state: null },
  reason: "Reaction wheel values",
  fixes,
});

it("候補のある行は操作を出し、候補の無い行は出さない", () => {
  const t = getT().fixes;
  const html = render(
    <MissingList
      design={undefined}
      missing={[
        withFixes([
          {
            kind: "set_value",
            path: "RW_X.max_momentum",
            target: { target: "attr", part: "RW_X", attr: "max_momentum", state: null },
            dimension: "AngularMomentum",
            command: "",
            catalog: null,
          },
          { kind: "add_list_item", list: "mode", command: "" },
        ]),
        { ...withFixes([]), path: "other", reason: "no fix" },
      ]}
    />,
  );
  expect(html).toContain(`>${t.enterValue}</button>`);
  expect(html).toContain(`>${t.addItem.mode}</a>`);
  expect(html.match(/<button/g)).toHaveLength(1);
});
