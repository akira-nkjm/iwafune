import { Outlet } from "react-router";
import { Frame } from "@/shell/Frame";

export default function AppLayout() {
  return (
    <Frame>
      <Outlet />
    </Frame>
  );
}
