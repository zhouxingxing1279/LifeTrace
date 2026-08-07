import { useEffect, useMemo, useRef, useState } from "react";
import { Command, CornerDownLeft, Search } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Kbd } from "@/src/components/ui";
import { filterCommandItems, groupCommandItems } from "./commandModel";

export interface CommandItem {
  id: string;
  label: string;
  hint?: string;
  icon?: LucideIcon;
  group: string;
  keywords?: string;
  execute: () => void;
}

export default function CommandPalette({
  open,
  onClose,
  items,
}: {
  open: boolean;
  onClose: () => void;
  items: CommandItem[];
}) {
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);

  const filtered = useMemo(
    () => filterCommandItems(items, query),
    [items, query],
  );

  const groups = useMemo(() => groupCommandItems(filtered), [filtered]);

  useEffect(() => {
    if (open) {
      setQuery("");
      setActiveIndex(0);
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const handler = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setActiveIndex((index) => Math.min(index + 1, filtered.length - 1));
        return;
      }
      if (event.key === "ArrowUp") {
        event.preventDefault();
        setActiveIndex((index) => Math.max(index - 1, 0));
        return;
      }
      if (event.key === "Enter" && filtered[activeIndex]) {
        event.preventDefault();
        filtered[activeIndex].execute();
        onClose();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, filtered, activeIndex, onClose]);

  useEffect(() => {
    const active = listRef.current?.querySelector<HTMLElement>(
      `[data-command-index="${activeIndex}"]`,
    );
    active?.scrollIntoView({ block: "nearest" });
  }, [activeIndex]);

  if (!open) return null;

  let flatIndex = 0;

  return (
    <div
      className="lt-command-backdrop"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
      role="presentation"
    >
      <div className="lt-command" role="dialog" aria-modal="true" aria-label="命令面板">
        <header>
          <Search aria-hidden="true" />
          <input
            autoFocus
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setActiveIndex(0);
            }}
            placeholder="搜索页面或执行操作…"
            aria-label="搜索命令"
          />
          <Kbd>Esc</Kbd>
        </header>
        <div className="lt-command-list" ref={listRef}>
          {groups.length === 0 ? (
            <p className="lt-command-empty">没有匹配的命令。</p>
          ) : (
            groups.map((group) => (
              <div key={group.group}>
                <div className="lt-command-group-label">{group.group}</div>
                {group.items.map((item) => {
                  const Icon = item.icon ?? Command;
                  const index = flatIndex++;
                  return (
                    <button
                      key={item.id}
                      type="button"
                      className="lt-command-item"
                      data-command-index={index}
                      data-active={activeIndex === index}
                      onMouseEnter={() => setActiveIndex(index)}
                      onClick={() => {
                        item.execute();
                        onClose();
                      }}
                    >
                      <Icon aria-hidden="true" />
                      <span>{item.label}</span>
                      {item.hint ? (
                        <span className="lt-command-hint">{item.hint}</span>
                      ) : (
                        <CornerDownLeft aria-hidden="true" className="lt-command-hint" />
                      )}
                    </button>
                  );
                })}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
