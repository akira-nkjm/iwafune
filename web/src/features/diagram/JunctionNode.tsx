import { Handle, type Node, type NodeProps, Position } from "@xyflow/react";

export type JunctionNodeType = Node<{ net: string }, "junction">;

/** 供給元もマスターも無い 3 点以上のネットの分岐点。ここにポートを落とすとそのネットに参加する */
export function JunctionNode({ data }: NodeProps<JunctionNodeType>) {
  return (
    <div className="size-3 rounded-full border-2 border-ink-3 bg-panel" title={data.net}>
      <Handle
        id="j"
        type="source"
        position={Position.Right}
        className="!inset-0 !size-3 !translate-0 !border-0 !bg-transparent"
      />
    </div>
  );
}
