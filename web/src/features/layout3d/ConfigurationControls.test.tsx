import { type ComponentProps, isValidElement, type ReactNode } from "react";
import { afterEach, expect, it, vi } from "vitest";
import type { ConfigurationText, ConfigurationView } from "@/api/types";
import { getT } from "@/i18n";
import { OpDialog } from "@/ops/OpDialog";
import { RenameDialog } from "@/ops/RenameDialog";
import { findElement, fire } from "@/test/elements";
import { ConfigurationControls } from "./ConfigurationControls";

const state = vi.hoisted(() => ({
  dialog: null as string | null,
  target: null as ConfigurationView | null,
  form: undefined as ConfigurationText | undefined,
  renamed: undefined as string | undefined,
  command: vi.fn(),
  apply: vi.fn(async (_request: unknown) => {}),
  select: vi.fn(),
}));

vi.mock("react", async (original) => ({
  ...(await original<typeof import("react")>()),
  useId: () => "configuration",
  useState: (initial: unknown) => {
    if (initial === null)
      return [
        state.target,
        (target: ConfigurationView) => {
          state.target = target;
        },
      ];
    if (typeof initial === "string")
      return [
        state.renamed ?? initial,
        (id: string) => {
          state.renamed = id;
        },
      ];
    return [
      state.form ?? initial,
      (form: ConfigurationText) => {
        state.form = form;
      },
    ];
  },
}));
vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
vi.mock("@/kit/detail/use-dialogs", () => ({
  useDialogs: () => ({
    current: state.dialog,
    show: (dialog: string) => {
      state.dialog = dialog;
    },
    props: { open: true, onOpenChange: vi.fn() },
  }),
}));
vi.mock("@/api/ops", () => ({
  useApplyOps: () => ({ mutateAsync: state.apply }),
  useCommandOps: (command: unknown) => {
    state.command(command);
    return { ops: null, error: null };
  },
}));
vi.mock("@/ops/use-op-check", () => ({ useOpCheck: () => ({ ok: true, failed: false }) }));

afterEach(() => {
  state.dialog = null;
  state.target = null;
  state.form = undefined;
  state.renamed = undefined;
  vi.clearAllMocks();
});

const stowed: ConfigurationView = {
  id: "STOWED",
  label: "Stowed",
  description: "Folded",
  overridden: [],
  text: { id: "STOWED", label: "Stowed", description: "Folded" },
  file: "configurations/STOWED.toml",
};
const controls = (selected: ConfigurationView | null = stowed) =>
  ConfigurationControls({ configurations: [stowed], selected, onSelect: state.select });
const text = getT().systems.layout3d.configuration;

function open(label: string, selected?: ConfigurationView | null) {
  const item = findElement(controls(selected), (props) => props.children === label)!;
  fire(item, "onSelect");
}

function form() {
  const element = findElement(controls(), (props) => props.open === true)!;
  const render = element.type;
  if (typeof render !== "function") throw new Error("形態のダイアログがありません");
  return (render as (props: typeof element.props) => ReactNode)(element.props);
}

function field(node: ReactNode, key: keyof ConfigurationText, value: string) {
  fire(findElement(node, (props) => props.id === `configuration-${key}`)!, "onChange", {
    target: { value },
  });
}

it("追加と編集のフォームは ID・表示名・説明をコマンドに渡す", () => {
  open(text.add, null);
  form();
  expect(state.command).toHaveBeenLastCalledWith(null);
  field(form(), "id", "DEPLOYED");
  field(form(), "label", "Deployed");
  field(form(), "description", "Open");
  form();
  expect(state.command).toHaveBeenLastCalledWith({
    command: "configuration_add",
    id: "DEPLOYED",
    label: "Deployed",
    description: "Open",
  });

  state.form = undefined;
  open(text.edit);
  expect(findElement(form(), (props) => props.id === "configuration-id")?.props.disabled).toBe(
    true,
  );
  field(form(), "label", "Launch");
  field(form(), "description", "");
  form();
  expect(state.command).toHaveBeenLastCalledWith({
    command: "configuration_set",
    id: "STOWED",
    label: "Launch",
    description: "",
  });
});

function dialogProps(node: ReactNode) {
  const dialog = findElement(node, (props) => "ops" in props)!;
  if (!isValidElement<ComponentProps<typeof OpDialog>>(dialog) || dialog.type !== OpDialog)
    throw new Error("共通の編集ダイアログがありません");
  return dialog.props;
}

async function submit(props: ComponentProps<typeof OpDialog>) {
  const form = findElement(OpDialog(props), (p) => typeof p.onSubmit === "function")!;
  fire(form, "onSubmit", { preventDefault: vi.fn() });
  await state.apply.mock.results[0]?.value;
}

it("ID の変更は RenameConfiguration を適用し、新しい ID を選ぶ", async () => {
  open(text.rename);
  const dialog = findElement(controls(), (props) => props.from === "STOWED")!;
  if (!isValidElement<ComponentProps<typeof RenameDialog>>(dialog) || dialog.type !== RenameDialog)
    throw new Error("ID の変更ダイアログがありません");
  const input = findElement(RenameDialog(dialog.props), (props) => props.id === "configuration")!;
  fire(input, "onChange", { target: { value: "LAUNCH" } });
  await submit(dialogProps(RenameDialog(dialog.props)));
  expect(state.apply).toHaveBeenCalledExactlyOnceWith({
    ops: [{ op: "rename_configuration", id: "STOWED", to: "LAUNCH" }],
    summary: text.renamed("STOWED", "LAUNCH"),
  });
  expect(state.select).toHaveBeenCalledExactlyOnceWith("LAUNCH", { replace: true });
});

it("削除は選んだ形態の RemoveConfiguration を適用する", async () => {
  open(text.remove);
  await submit(dialogProps(controls()));
  expect(state.apply).toHaveBeenCalledExactlyOnceWith({
    ops: [{ op: "remove_configuration", id: "STOWED" }],
    summary: text.removeTitle("STOWED"),
  });
  expect(state.select).toHaveBeenCalledExactlyOnceWith(null, { replace: true });
  // 消した後の再読込が消した形態を読みに行かないよう、適用の前に基準へ移る
  expect(state.select.mock.invocationCallOrder[0]).toBeLessThan(
    state.apply.mock.invocationCallOrder[0]!,
  );
});

it("切り替え欄は形態 ID を選び、基準を選ぶと指定を外す", () => {
  const select = (selected: ConfigurationView | null) =>
    findElement(controls(selected), (props) => props["aria-label"] === text.select)!;
  fire(select(null), "onChange", { target: { value: "STOWED" } });
  fire(select(stowed), "onChange", { target: { value: "" } });
  expect(state.select.mock.calls).toEqual([["STOWED"], [null]]);
});
