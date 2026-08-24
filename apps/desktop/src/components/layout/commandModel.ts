import type { CommandItem } from "./CommandPalette";

const normalize = (value: string) => value.toLowerCase().trim();

export function filterCommandItems(
  items: CommandItem[],
  query: string,
): CommandItem[] {
  const needle = normalize(query);
  if (!needle) return items;
  return items.filter((item) =>
    normalize(`${item.label} ${item.hint ?? ""} ${item.keywords ?? ""}`).includes(
      needle,
    ),
  );
}

export function groupCommandItems(
  items: CommandItem[],
): { group: string; items: CommandItem[] }[] {
  const result: { group: string; items: CommandItem[] }[] = [];
  for (const item of items) {
    const last = result[result.length - 1];
    if (last && last.group === item.group) last.items.push(item);
    else result.push({ group: item.group, items: [item] });
  }
  return result;
}
