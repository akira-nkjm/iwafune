import { House, RotateCcw, RotateCw, ZoomIn, ZoomOut } from "lucide-react";
import { useT } from "@/i18n";
import { ViewHelp } from "@/kit/view/ViewHelp";
import { ViewButton, ViewToolbar } from "@/kit/view/ViewToolbar";
import { NOTCH_ZOOM_STEP } from "@/lib/wheel-input";

/**
 * 3D の視点操作。右上に縮小・拡大・ホームポジション・操作の案内、右下のビューキューブの右上に、今見ている向きを軸に画面を回す
 * 2 つの曲がった矢印（`cube` が false ならビューキューブが無いので出さない）。（全体を収めて初めの向きへ）・操作の案内。図の左下の帯と同じ見た目 */
export function ViewControls3D({
  onZoom,
  onRoll,
  onHome,
  cube = true,
}: {
  /** 画面の中心へ向けた拡大率（>1 で拡大）を渡す */
  onZoom: (factor: number) => void;
  /** 視線を軸に画面を 1 刻み回す（1 = 時計回り、−1 = 反時計回り。角は見ている向きで決まる） */
  onRoll: (turn: 1 | -1) => void;
  onHome: () => void;
  /** ビューキューブを出しているか */
  cube?: boolean;
}) {
  const L = useT().systems.layout3d;
  return (
    <>
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
      {cube && (
        // ビューキューブ（右下、中心が端から 64 px、対角の半分が約 52 px）の右上
        <div className="absolute right-2 bottom-[112px] flex items-center">
          <ViewButton label={L.rollLeft} onClick={() => onRoll(-1)}>
            <RotateCcw size={15} aria-hidden />
          </ViewButton>
          <ViewButton label={L.rollRight} onClick={() => onRoll(1)}>
            <RotateCw size={15} aria-hidden />
          </ViewButton>
        </div>
      )}
    </>
  );
}
