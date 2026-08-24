
import { useRef, useState } from "react";
import { MoreHorizontal } from "lucide-react";
import type { AppAction } from "@/src/ui/actions/types";
import ActionMenu from "./ActionMenu";

interface MoreMenuProps<TContext> {
  actions: AppAction<TContext>[];
  context: TContext;
  label?: string;
  buttonClassName?: string;
}

export default function MoreMenu<TContext>({
  actions,
  context,
  label = "更多操作",
  buttonClassName = "",
}: MoreMenuProps<TContext>) {
  const buttonRef = useRef<HTMLButtonElement>(null);
  const [anchor, setAnchor] = useState<{ x: number; y: number } | null>(null);

  return (
    <>
      <button
        ref={buttonRef}
        type="button"
        className={`ui-menu-trigger ${buttonClassName}`.trim()}
        aria-label={label}
        aria-haspopup="menu"
        aria-expanded={Boolean(anchor)}
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          const rect = buttonRef.current?.getBoundingClientRect();
          if (!rect) return;
          setAnchor((current) => current ? null : { x: rect.right - 212, y: rect.bottom + 6 });
        }}
      >
        <MoreHorizontal aria-hidden="true" />
      </button>
      {anchor ? (
        <ActionMenu
          actions={actions}
          context={context}
          anchor={anchor}
          ariaLabel={label}
          onClose={() => setAnchor(null)}
        />
      ) : null}
    </>
  );
}
