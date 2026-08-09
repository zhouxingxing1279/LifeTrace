import { useEffect, type ComponentType, type SVGProps } from "react";

type Icon = ComponentType<SVGProps<SVGSVGElement>>;
export type ExecutionMenuItem = {
  id: string;
  label: string;
  icon?: Icon;
  danger?: boolean;
  disabled?: boolean;
  action: () => void;
};

type Props = {
  x: number;
  y: number;
  items: ExecutionMenuItem[];
  onClose: () => void;
};

export default function ExecutionContextMenu({ x, y, items, onClose }: Props) {
  useEffect(() => {
    const close = () => onClose();
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("pointerdown", close);
    window.addEventListener("blur", close);
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("pointerdown", close);
      window.removeEventListener("blur", close);
      window.removeEventListener("keydown", key);
    };
  }, [onClose]);

  const left = Math.min(x, Math.max(12, window.innerWidth - 228));
  const top = Math.min(y, Math.max(12, window.innerHeight - Math.max(80, items.length * 38 + 16)));

  return <div className="lt-exec-context-menu" role="menu" style={{ left, top }} onPointerDown={(event) => event.stopPropagation()}>
    {items.map((item) => {
      const Icon = item.icon;
      return <button
        key={item.id}
        type="button"
        role="menuitem"
        className={item.danger ? "danger" : ""}
        disabled={item.disabled}
        onClick={() => { item.action(); onClose(); }}
      >{Icon ? <Icon aria-hidden="true"/> : null}<span>{item.label}</span></button>;
    })}
  </div>;
}
