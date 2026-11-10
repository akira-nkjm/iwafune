import { type Node, type NodeProps, NodeResizer } from "@xyflow/react";
import { toneBorder, toneText, toneTint } from "@/lib/tone";
import type { DiagramContainer } from "./model";

export type ContainerNodeType = Node<
  { container: DiagramContainer; onResized: (id: string, w: number, h: number) => void },
  "container"
>;

/** サブシステムの枠（parts/ のディレクトリ）。淡い地と色の枠、右上に名前。角を引いて大きさを変えられる */
export function ContainerNode({ id, data, width, height, selected }: NodeProps<ContainerNodeType>) {
  const c = data.container;
  return (
    <div
      className={`relative h-full w-full rounded-[13px] border ${toneBorder[c.tone]}`}
      style={{ width, height, background: toneTint(c.tone, "frame") }}
    >
      <NodeResizer
        isVisible={selected}
        minWidth={160}
        minHeight={90}
        lineClassName="!border-accent"
        handleClassName="!size-2 !bg-accent"
        onResizeEnd={(_, p) => data.onResized(id, p.width, p.height)}
      />
      <div className={`absolute top-1.5 right-3 font-bold text-[13px] ${toneText[c.tone]}`}>
        {c.label}
      </div>
    </div>
  );
}
