import { House, ZoomIn, ZoomOut } from "lucide-react";
import { useT } from "@/i18n";
import { ViewHelp } from "@/kit/view/ViewHelp";
import { ViewButton, ViewToolbar } from "@/kit/view/ViewToolbar";
import { NOTCH_ZOOM_STEP } from "@/lib/wheel-input";

/** 3D の右上の視点操作: 縮小・拡大・ホームポジション（全体を収めて初めの向きへ）・操作の案内。図の左下の帯と同じ見た目 */
export function ViewControls3D({
  onZoom,
  onHome,
}: {
  /** 画面の中心へ向けた拡大率（>1 で拡大）を渡す */
  onZoom: (factor: number) => void;
  onHome: () => void;
}) {
  const L = useT().systems.layout3d;
  return (
    <ViewToolbar className="absolute top-3 right-3 bg-panel/90">
      <ViewButton label={L.zoomOut} onClick={() => onZoom(1 / NOTCH_ZOOM_STEP)}>
        <ZoomOut size={14} aria-hidden />
      </ViewButton>
      <ViewButton label={L.zoomIn} onClick={() => onZoom(NOTCH_ZOOM_STEP)}>
        <ZoomIn size={14} aria-hidden />
      </ViewButton>
      <ViewButton label={L.home} onClick={onHome}>
        <House size={14} aria-hidden />
      </ViewButton>
      <ViewHelp view="3d" />
    </ViewToolbar>
  );
}
