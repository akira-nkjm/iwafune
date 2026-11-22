import { CircleQuestionMark } from "lucide-react";
import { useT } from "@/i18n";
import { Popover, PopoverContent, PopoverTrigger } from "@/ui/popover";
import { ViewButton } from "./ViewToolbar";

/** 視点操作の帯に置く「?」。押すと、この画面のトラックパッドとマウスの操作を一覧する（キーボードで開閉できる）。`view` は図か 3D か */
export function ViewHelp({ view }: { view: "diagram" | "3d" }) {
  const L = useT().common.viewHelp;
  const rows =
    view === "3d"
      ? [
          { action: L.zoom, how: L.zoomHow },
          { action: L.pan, how: L.pan3dHow },
          { action: L.rotate, how: L.rotateHow },
          { action: L.fit, how: L.fit3dHow },
        ]
      : [
          { action: L.zoom, how: L.zoomHow },
          { action: L.pan, how: L.panDiagramHow },
          { action: L.fit, how: L.fitDiagramHow },
        ];
  return (
    <Popover>
      <PopoverTrigger asChild>
        <ViewButton label={L.label}>
          <CircleQuestionMark size={14} aria-hidden />
        </ViewButton>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-80 gap-2 text-xs">
        <p className="font-medium text-ink">{L.title}</p>
        <dl className="grid grid-cols-[4.5rem_minmax(0,1fr)] gap-x-2 gap-y-1.5 text-ink-2">
          {rows.map((r) => (
            <div key={r.action} className="contents">
              <dt className="font-medium text-ink">{r.action}</dt>
              <dd>{r.how}</dd>
            </div>
          ))}
        </dl>
      </PopoverContent>
    </Popover>
  );
}
