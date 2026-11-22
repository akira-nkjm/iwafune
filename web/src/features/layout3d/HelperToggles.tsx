import { useT } from "@/i18n";
import { Toggle } from "@/kit/selection/Toggle";
import { type SceneHelpers, visibleHelpers } from "./visibility";

/** 視野と座標の表示（原点の軸・地面の格子・画面隅の軸）を 1 つずつ出し入れするボタンの組（3D の操作の帯。配置でも熱画像でも出す） */
export function HelperToggles({
  value,
  onChange,
  hasSun = false,
}: {
  value: SceneHelpers;
  onChange: (value: SceneHelpers) => void;
  /** 太陽の矢印のボタンを出すか（太陽方向を渡す熱の非定常だけ） */
  hasSun?: boolean;
}) {
  const L = useT().systems.layout3d;
  return (
    <fieldset className="flex shrink-0 items-center gap-1">
      <legend className="sr-only">{L.helpers}</legend>
      {visibleHelpers(hasSun).map((key) => (
        <Toggle
          key={key}
          value={value[key]}
          onChange={(on) => onChange({ ...value, [key]: on })}
          className="h-6 px-2 text-2xs"
        >
          {L.helper[key]}
        </Toggle>
      ))}
    </fieldset>
  );
}
