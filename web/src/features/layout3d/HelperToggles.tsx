import { useT } from "@/i18n";
import { Toggle } from "@/kit/selection/Toggle";
import { SCENE_HELPERS, type SceneHelpers } from "./visibility";

/** 座標の表示（原点の軸・地面の格子・画面隅の軸）を 1 つずつ出し入れするボタンの組（配置のページの操作の帯） */
export function HelperToggles({
  value,
  onChange,
}: {
  value: SceneHelpers;
  onChange: (value: SceneHelpers) => void;
}) {
  const L = useT().systems.layout3d;
  return (
    <fieldset className="flex shrink-0 items-center gap-1">
      <legend className="sr-only">{L.helpers}</legend>
      {SCENE_HELPERS.map((key) => (
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
