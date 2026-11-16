import { MutationObserver, QueryClient } from "@tanstack/react-query";
import { type ComponentProps, isValidElement, type ReactElement, type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { OpsRequest } from "@/api/ops";
import type { ApplyResult, CatalogItemView, DesignView } from "@/api/types";
import { getT } from "@/i18n";
import { ListDetailPage } from "@/kit/list-detail/ListDetailPage";
import { paths } from "@/lib/nav";
import { OpDialog } from "@/ops/OpDialog";
import { RenameDialog } from "@/ops/RenameDialog";
import { findElement, fire } from "@/test/elements";
import { fixture } from "@/test/fixture";
import { CatalogDetail } from "../catalog/CatalogDetail";
import { CatalogPage } from "../catalog/CatalogPage";
import { RenameCatalogDialog } from "../catalog/RenameCatalogDialog";
import { ComponentsPage } from "./ComponentsPage";
import { DeletePartDialog } from "./DeletePartDialog";
import { PartDetail } from "./detail/PartDetail";
import { RenamePartDialog } from "./RenamePartDialog";

const state = vi.hoisted(() => ({
  navigate: vi.fn(),
  close: vi.fn(),
  dialog: "rename",
  newId: "",
}));

// DOM のない既存のテストと同じく、要素のハンドラーを直接呼ぶ。
// hooks の状態だけを置き換え、ページ → 詳細 → ダイアログ → URL の接続を使う。
vi.mock("react", async (original) => ({
  ...(await original<typeof import("react")>()),
  useId: () => "new-id",
  useState: (initial: unknown) => [
    typeof initial === "string" && initial ? state.newId : initial,
    vi.fn(),
  ],
}));
vi.mock("react-router", async (original) => ({
  ...(await original<typeof import("react-router")>()),
  useNavigate: () => state.navigate,
  useParams: () => ({ id: "old" }),
  useSearchParams: () => [new URLSearchParams("view=tree&new=1"), vi.fn()],
}));
vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
vi.mock("@tanstack/react-query", async (original) => ({
  ...(await original<typeof import("@tanstack/react-query")>()),
  useQuery: () => ({ data: undefined }),
}));
vi.mock("@/api/queries", async (original) => ({
  ...(await original<typeof import("@/api/queries")>()),
  useDesign: () => ({ data: fixture }),
}));
vi.mock("@/kit/detail/use-dialogs", () => ({
  useDialogs: () => ({ current: state.dialog, props: { open: true, onOpenChange: state.close } }),
}));
vi.mock("@/ops/use-op-check", () => ({
  useOpCheck: () => ({ ok: true, error: null }),
}));
vi.mock("@/api/ops", () => ({ useApplyOps: () => mutation() }));

let observer: MutationObserver<ApplyResult, Error, OpsRequest>;
let unsubscribe: () => void;
let resolve: (result: ApplyResult) => void;
let reject: (error: Error) => void;
let settled: Promise<unknown>;
let sent: OpsRequest | undefined;

function mutation() {
  const result = observer.getCurrentResult();
  return {
    ...result,
    mutateAsync: (request: OpsRequest) => {
      settled = observer.mutate(request);
      return settled;
    },
    mutate: (request: OpsRequest, options: Parameters<typeof observer.mutate>[1]) => {
      settled = observer.mutate(request, options);
      void settled.catch(() => {});
    },
  };
}

const applied: ApplyResult = {
  revision: "after",
  previous_revision: "before",
  dry_run: false,
  changed: [],
  diagnostics: [],
  cli: [],
};
const catalog: CatalogItemView = {
  id: "local/old",
  name: "old",
  scope: "local",
  dir: [],
  editable: true,
  kind: "wheel",
  system: null,
  label: null,
  label_en: null,
  manufacturer: null,
  part_number: null,
  maturity: null,
  tags: [],
  tags_text: "",
  datasheet: null,
  datasheet_link: null,
  source: null,
  ports: {},
  states: [],
  values: {},
  paths: {},
  users: [],
};

/** 関数の部品を展開せず、指定した型の要素を取り出す */
function element<P>(node: ReactNode, type: (props: P) => ReactNode): ReactElement<P> {
  for (const child of Array.isArray(node) ? node : [node]) {
    if (!isValidElement<{ children?: ReactNode }>(child)) continue;
    if (child.type === type) return child as ReactElement<P>;
    try {
      return element(child.props.children, type);
    } catch {
      /* 次の子を探す */
    }
  }
  throw new Error("要素がありません");
}

function dialogFor(kind: "part" | "catalog", rename: boolean) {
  const part = fixture.parts[0];
  if (!part) throw new Error("部品がありません");
  const design: DesignView = { ...fixture, catalog: [catalog] };
  const page = kind === "part" ? ComponentsPage() : CatalogPage();
  const layout = element(page, ListDetailPage);
  const detail = layout.props.detail(design, kind === "part" ? part.id : catalog.id);
  if (kind === "part") {
    const tree = PartDetail(element(detail, PartDetail).props);
    if (!rename)
      return element(DeletePartDialog(element(tree, DeletePartDialog).props), OpDialog).props;
    const wrapper = RenamePartDialog(element(tree, RenamePartDialog).props);
    return element(RenameDialog(element(wrapper, RenameDialog).props), OpDialog).props;
  }
  const tree = CatalogDetail(element(detail, CatalogDetail).props);
  if (!rename) return element(tree, OpDialog).props;
  const wrapper = RenameCatalogDialog(element(tree, RenameCatalogDialog).props);
  return element(RenameDialog(element(wrapper, RenameDialog).props), OpDialog).props;
}

function submit(props: ComponentProps<typeof OpDialog>) {
  const tree = OpDialog(props);
  const form = findElement(tree, (p) => typeof p.onSubmit === "function");
  if (!form) throw new Error("フォームがありません");
  fire(form, "onSubmit", { preventDefault: vi.fn() });
  return tree;
}

beforeEach(() => {
  vi.clearAllMocks();
  observer = new MutationObserver(new QueryClient(), {
    gcTime: 0,
    mutationFn: (request: OpsRequest) => {
      sent = request;
      return new Promise<ApplyResult>((done, fail) => {
        resolve = done;
        reject = fail;
      });
    },
    // useApplyOps の onApplied が設計を再読込し、古い ID の詳細が unmount する順序を再現。
    // unsubscribe 後は mutate に渡した onSuccess が呼ばれなくなる。
    onSuccess: () => unsubscribe(),
  });
  unsubscribe = observer.subscribe(() => {});
  sent = undefined;
});
afterEach(() => unsubscribe());

describe("詳細からの名前変更・削除後の URL", () => {
  for (const kind of ["part", "catalog"] as const) {
    for (const rename of [true, false]) {
      it(`${kind}: ${rename ? "名前変更後は新 ID の詳細" : "削除後は一覧"}に移る（詳細が消えても実行する）`, async () => {
        state.dialog = rename ? "rename" : kind === "part" ? "delete" : "remove";
        state.newId = kind === "part" ? "new.part%" : "local/new.item%";
        submit(dialogFor(kind, rename));
        await vi.waitFor(() => expect(sent).toBeDefined());
        expect(state.navigate).not.toHaveBeenCalled();
        resolve(applied);
        await settled;
        await vi.waitFor(() => expect(state.navigate).toHaveBeenCalledOnce());
        const pathname =
          kind === "part"
            ? rename
              ? paths.component(state.newId)
              : paths.components()
            : rename
              ? paths.catalogItem(state.newId)
              : paths.catalog();
        expect(state.navigate).toHaveBeenCalledWith({ pathname, search: "view=tree" });
        expect(state.close).toHaveBeenCalledWith(false);
        expect(sent?.ops).toEqual([
          kind === "part"
            ? rename
              ? { op: "rename_part", id: fixture.parts[0]?.id, to: state.newId }
              : { op: "remove_part", id: fixture.parts[0]?.id }
            : rename
              ? { op: "rename_catalog_item", from: catalog.id, to: state.newId }
              : { op: "remove_catalog_item", id: catalog.id },
        ]);
      });
    }
  }

  it("失敗したら閉じず、遷移せず、エラーを出す", async () => {
    state.dialog = "delete";
    const props = dialogFor("part", false);
    submit(props);
    await vi.waitFor(() => expect(sent).toBeDefined());
    reject(new Error("operation failed"));
    await expect(settled).rejects.toThrow("operation failed");
    await Promise.resolve();
    expect(state.close).not.toHaveBeenCalled();
    expect(state.navigate).not.toHaveBeenCalled();
    const error = findElement(OpDialog(props), (p) => p.children === "operation failed");
    expect(error).not.toBeNull();
  });

  it("onDone が無い利用箇所も成功時に閉じる", async () => {
    submit({
      open: true,
      onOpenChange: state.close,
      title: "Apply",
      summary: "Apply",
      ops: [{ op: "remove_catalog_item", id: catalog.id }],
    });
    await vi.waitFor(() => expect(sent).toBeDefined());
    resolve(applied);
    await settled;
    expect(state.close).toHaveBeenCalledWith(false);
  });
});
