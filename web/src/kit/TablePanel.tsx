import type { ReactNode } from "react";
import { Panel } from "./Panel";
import { PanelHeader } from "./PanelHeader";

/**
 * 見出しの下に全幅の表を置くパネル。見出し（`PanelHeader` と同じ `title`・`caption`・`right`）だけを内側に寄せ、
 * 中身（`DataTable` など）はパネルの端まで使う
 */
export function TablePanel({
  title,
  caption,
  right,
  className,
  children,
}: {
  title: ReactNode;
  caption?: ReactNode;
  right?: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  return (
    <Panel pad={false} className={className}>
      <div className="p-4 pb-2">
        <PanelHeader className="mb-0" title={title} caption={caption} right={right} />
      </div>
      {children}
    </Panel>
  );
}
