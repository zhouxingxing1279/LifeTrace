
import type { CSSProperties, ElementType, ReactNode } from "react";
import { useMemo, useState } from "react";
import { resolveActions } from "@/src/ui/actions/resolveActions";
import type { AppAction } from "@/src/ui/actions/types";
import ActionMenu from "./ActionMenu";

interface ContextMenuProps<TContext> {
  as?: ElementType;
  actions: AppAction<TContext>[];
  context: TContext;
  children: ReactNode;
  className?: string;
  style?: CSSProperties;
  ariaLabel?: string;
}

const nativeMenuSelector = [
  "input",
  "textarea",
  "select",
  "[contenteditable='true']",
  ".ProseMirror",
].join(",");

export default function ContextMenu<TContext>({
  as: Element = "div",
  actions,
  context,
  children,
  className,
  style,
  ariaLabel,
}: ContextMenuProps<TContext>) {
  const [anchor, setAnchor] = useState<{ x: number; y: number } | null>(null);
  const available = useMemo(() => resolveActions(actions, context), [actions, context]);

  return (
    <>
      <Element
        className={className}
        style={style}
        onContextMenu={(event: React.MouseEvent<HTMLElement>) => {
          const target = event.target as HTMLElement;
          if (target.closest(nativeMenuSelector) || available.length === 0) return;
          event.preventDefault();
          event.stopPropagation();
          setAnchor({ x: event.clientX, y: event.clientY });
        }}
      >
        {children}
      </Element>
      {anchor ? (
        <ActionMenu
          actions={actions}
          context={context}
          anchor={anchor}
          ariaLabel={ariaLabel}
          onClose={() => setAnchor(null)}
        />
      ) : null}
    </>
  );
}
