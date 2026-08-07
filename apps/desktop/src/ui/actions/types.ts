
import type { LucideIcon } from "lucide-react";

export type ActionGroup = "primary" | "related" | "organize" | "danger";

export interface AppAction<TContext = unknown> {
  id: string;
  label: string;
  icon?: LucideIcon;
  shortcut?: string;
  group?: ActionGroup;
  danger?: boolean;
  hidden?: boolean | ((context: TContext) => boolean);
  disabled?: boolean | ((context: TContext) => boolean);
  children?: AppAction<TContext>[];
  execute: (context: TContext) => void | Promise<void>;
}
