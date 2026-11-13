import { type ReactNode, useEffect, useState } from "react";
import { useT } from "@/i18n";

/** 節の位置を追う細い目次。スクロール先はページ内の要素だけ。 */
export function PowerOutline({
  items,
  children,
}: {
  items: { id: string; label: string }[];
  children: ReactNode;
}) {
  const [active, setActive] = useState(items[0]?.id);
  const t = useT().systems.power;
  useEffect(() => {
    if (typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries.find((entry) => entry.isIntersecting);
        if (visible) setActive(visible.target.id);
      },
      { rootMargin: "-10% 0px -65% 0px" },
    );
    for (const item of items) {
      const section = document.getElementById(item.id);
      if (section) observer.observe(section);
    }
    return () => observer.disconnect();
  }, [items]);
  return (
    <div className="relative grid gap-6 lg:grid-cols-[minmax(0,1fr)_150px]">
      <div className="min-w-0 space-y-5">{children}</div>
      <nav
        aria-label={t.outline}
        className="sticky top-6 hidden h-fit border-l border-line pl-3 lg:block"
      >
        {items.map((item) => (
          <button
            type="button"
            key={item.id}
            aria-current={active === item.id ? "location" : undefined}
            onClick={() => {
              setActive(item.id);
              document
                .getElementById(item.id)
                ?.scrollIntoView({ behavior: "smooth", block: "start" });
            }}
            className="group flex w-full items-center gap-2 py-2 text-left text-xs text-ink-3 aria-current:text-accent"
          >
            <span className="h-px w-3 shrink-0 bg-line-strong group-aria-current:w-5 group-aria-current:bg-accent" />
            {item.label}
          </button>
        ))}
      </nav>
    </div>
  );
}
