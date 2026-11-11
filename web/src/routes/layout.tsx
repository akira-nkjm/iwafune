import { Outlet } from "react-router";
import { Frame } from "@/shell/Frame";
import { preloadPlan } from "./pages";

/** 画面の枠。何を先に読むか（ページの本体・系の解析など）はページの登録（pages.ts）から渡す */
export default function AppLayout() {
  return (
    <Frame preload={preloadPlan}>
      <Outlet />
    </Frame>
  );
}
