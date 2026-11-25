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
            example: null,
            choices: [],
            catalog: null,
          },
          { kind: "add_list_item", list: "mode", id: null, command: "" },
          { kind: "add_list_item", list: "case", id: "HOT", command: "" },
        ]),
        { ...withFixes([]), path: "other", reason: "no fix" },
      ]}
    />,
  );
  expect(html).toContain(`>${t.enterValue}</button>`);
  // どの一覧（モード・ケース）の追加も、その場のダイアログを開くボタン
  expect(html).toContain(`>${t.addItem.mode}</button>`);
  expect(html).toContain(`>${t.addItem.case}</button>`);
  expect(html.match(/<button/g)).toHaveLength(3);
});

it("カタログ品を指す欄の行は、カタログ品を選ぶ操作を出す", () => {
  const t = getT().fixes;
  const html = render(
    <MissingList
      design={undefined}
      missing={[
        withFixes([
          {
            kind: "pick_ref",
            target: {
              kind: "param",
              target: "setting",
              scope: "conditions",
              segments: ["launch", "launcher"],
            },
            path: "conditions.launch.launcher",
            what: "catalog_item",
            roles: [],
            kinds: ["launcher"],
            choices: [{ value: "local/lv_alpha", label: "local/lv_alpha", label_en: null }],
            command: "fune set conditions.launch.launcher <item>",
            add_command: null,
          },
        ]),
      ]}
    />,
  );
  expect(html).toContain(`>${t.pick.action.catalog_item}</button>`);
});

it("役割の部品を足す・割り当てる・部品に値を入れる候補は、その場のダイアログを開くボタンにする", () => {
  const t = getT().fixes;
  const html = render(
    <MissingList
      design={undefined}
      missing={[
        withFixes([
          {
            kind: "add_part",
            roles: ["battery"],
            kinds: ["battery"],
            catalog_items: [],
            command: "",
          },
          {
            kind: "assign_part",
            roles: ["battery"],
            kinds: ["battery"],
            parts: ["OBC"],
            command: "",
          },
          {
            kind: "set_value_on_part",
            attr: "capacity",
            dimension: null,
            example: null,
            choices: [],
            parts: ["BAT"],
            command: "",
          },
        ]),
      ]}
    />,
  );
  expect(html).toContain(`>${t.addPart}</button>`);
  expect(html).toContain(`>${t.assignPart}</button>`);
  expect(html).toContain(`>${t.setOnPart}</button>`);
  expect(html).not.toContain("<a ");
});
