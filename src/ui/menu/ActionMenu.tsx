
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { ChevronRight } from "lucide-react";
import { isActionDisabled, resolveActions } from "@/src/ui/actions/resolveActions";
import type { ActionGroup, AppAction } from "@/src/ui/actions/types";
import { clampMenuPosition } from "./menuPosition";

interface ActionMenuProps<TContext> {
  actions: AppAction<TContext>[];
  context: TContext;
  anchor: { x: number; y: number };
  onClose: () => void;
  ariaLabel?: string;
}

const groupOrder: ActionGroup[] = ["primary", "related", "organize", "danger"];

export default function ActionMenu<TContext>({
  actions,
  context,
  anchor,
  onClose,
  ariaLabel = "操作菜单",
}: ActionMenuProps<TContext>) {
  const menuRef = useRef<HTMLDivElement>(null);
  const resolved = useMemo(() => resolveActions(actions, context), [actions, context]);
  const flatActions = useMemo(
    () => resolved.flatMap((action) => [action, ...(action.children ?? [])]),
    [resolved],
  );
  const [activeIndex, setActiveIndex] = useState(() => {
    const index = flatActions.findIndex((action) => !isActionDisabled(action, context));
    return index < 0 ? 0 : index;
  });
  const [position, setPosition] = useState(anchor);

  useLayoutEffect(() => {
    const element = menuRef.current;
    if (!element) return;
    const rect = element.getBoundingClientRect();
    setPosition(clampMenuPosition({
      x: anchor.x,
      y: anchor.y,
      width: rect.width,
      height: rect.height,
      viewportWidth: window.innerWidth,
      viewportHeight: window.innerHeight,
    }));
    element.focus({ preventScroll: true });
  }, [anchor]);

  useEffect(() => {
    const closeOnPointer = (event: PointerEvent) => {
      if (!menuRef.current?.contains(event.target as Node)) onClose();
    };
    const closeOnViewportChange = () => onClose();
    document.addEventListener("pointerdown", closeOnPointer, true);
    window.addEventListener("resize", closeOnViewportChange);
    window.addEventListener("scroll", closeOnViewportChange, true);
    return () => {
      document.removeEventListener("pointerdown", closeOnPointer, true);
      window.removeEventListener("resize", closeOnViewportChange);
      window.removeEventListener("scroll", closeOnViewportChange, true);
    };
  }, [onClose]);

  const enabledIndexes = flatActions
    .map((action, index) => ({ action, index }))
    .filter(({ action }) => !isActionDisabled(action, context))
    .map(({ index }) => index);

  const move = (direction: 1 | -1) => {
    if (!enabledIndexes.length) return;
    const current = enabledIndexes.indexOf(activeIndex);
    const next = current < 0 ? 0 : (current + direction + enabledIndexes.length) % enabledIndexes.length;
    setActiveIndex(enabledIndexes[next]);
  };

  const execute = async (action: AppAction<TContext>) => {
    if (isActionDisabled(action, context) || action.children?.length) return;
    await action.execute(context);
    onClose();
  };

  const groups = groupOrder
    .map((group) => ({ group, actions: resolved.filter((action) => (action.group ?? "related") === group) }))
    .filter(({ actions: items }) => items.length > 0);

  if (typeof document === "undefined") return null;

  return createPortal(
    <div
      ref={menuRef}
      className="ui-action-menu"
      style={{ left: position.x, top: position.y }}
      role="menu"
      aria-label={ariaLabel}
      tabIndex={-1}
      onContextMenu={(event) => event.preventDefault()}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          onClose();
        } else if (event.key === "ArrowDown") {
          event.preventDefault();
          move(1);
        } else if (event.key === "ArrowUp") {
          event.preventDefault();
          move(-1);
        } else if (event.key === "Home" && enabledIndexes.length) {
          event.preventDefault();
          setActiveIndex(enabledIndexes[0]);
        } else if (event.key === "End" && enabledIndexes.length) {
          event.preventDefault();
          setActiveIndex(enabledIndexes[enabledIndexes.length - 1]);
        } else if ((event.key === "Enter" || event.key === " ") && flatActions[activeIndex]) {
          event.preventDefault();
          void execute(flatActions[activeIndex]);
        }
      }}
    >
      {groups.length === 0 ? <div className="ui-action-menu-empty">没有可用操作</div> : groups.map(({ group, actions: items }) => (
        <div className="ui-action-menu-group" role="group" key={group}>
          {items.flatMap((action) => [action, ...(action.children ?? [])]).map((action) => {
            const index = flatActions.indexOf(action);
            const Icon = action.icon;
            const disabled = isActionDisabled(action, context);
            return (
              <button
                type="button"
                className="ui-action-menu-item"
                role="menuitem"
                key={`${group}-${action.id}`}
                disabled={disabled}
                data-active={activeIndex === index}
                data-danger={action.danger || action.group === "danger"}
                onPointerMove={() => setActiveIndex(index)}
                onClick={() => void execute(action)}
              >
                <span>{Icon ? <Icon size={16} aria-hidden="true" /> : null}</span>
                <span>{action.label}</span>
                {action.children?.length ? <ChevronRight size={15} aria-hidden="true" /> : <kbd>{action.shortcut ?? ""}</kbd>}
              </button>
            );
          })}
        </div>
      ))}
    </div>,
    document.body,
  );
}
