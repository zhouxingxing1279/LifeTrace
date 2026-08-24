
import type { AppAction } from "./types";

function resolveFlag<TContext>(
  flag: boolean | ((context: TContext) => boolean) | undefined,
  context: TContext,
): boolean {
  return typeof flag === "function" ? flag(context) : Boolean(flag);
}

export function isActionDisabled<TContext>(action: AppAction<TContext>, context: TContext): boolean {
  return resolveFlag(action.disabled, context);
}

export function resolveActions<TContext>(
  actions: AppAction<TContext>[],
  context: TContext,
): AppAction<TContext>[] {
  return actions
    .filter((action) => !resolveFlag(action.hidden, context))
    .map((action) => ({
      ...action,
      children: action.children ? resolveActions(action.children, context) : undefined,
    }));
}
