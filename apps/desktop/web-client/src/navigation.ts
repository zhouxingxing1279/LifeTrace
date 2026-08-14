export type Route =
  | "/"
  | "/assistant"
  | "/execution"
  | "/execution/control"
  | "/habits"
  | "/english/articles"
  | "/english/vocabulary"
  | "/english/stats"
  | "/fitness"
  | "/notes"
  | "/calendar"
  | "/review"
  | "/finance"
  | "/finance/transactions"
  | "/finance/accounts"
  | "/finance/categories"
  | "/finance/budgets"
  | "/finance/import"
  | "/finance/beecount"
  | "/devices"
  | "/settings"
  | "/search";

export type NavIcon =
  | "home"
  | "bot"
  | "check"
  | "languages"
  | "dumbbell"
  | "note"
  | "calendar"
  | "review"
  | "chart"
  | "money"
  | "wallet"
  | "upload"
  | "cloud"
  | "devices"
  | "settings"
  | "search";

export interface NavItem {
  route: Route;
  label: string;
  icon: NavIcon;
}

export interface NavGroup {
  label: string;
  items: NavItem[];
}

/*
 * Sidebar IA intentionally exposes destinations, not every sub-route.
 * Finance, English and Execution detail routes live inside their own local
 * navigation, so the global navigation stays readable as LifeTrace grows.
 */
export const NAV_GROUPS: NavGroup[] = [
  {
    label: "工作台",
    items: [
      { route: "/", label: "今日总览", icon: "home" },
      { route: "/execution", label: "计划与待办", icon: "check" },
      { route: "/assistant", label: "AI 管家", icon: "bot" },
    ],
  },
  {
    label: "成长健康",
    items: [
      { route: "/habits", label: "坚持项目", icon: "check" },
      { route: "/fitness", label: "健身训练", icon: "dumbbell" },
      { route: "/calendar", label: "生活日历", icon: "calendar" },
      { route: "/review", label: "每日复盘", icon: "review" },
    ],
  },
  {
    label: "知识学习",
    items: [
      { route: "/notes", label: "笔记", icon: "note" },
      { route: "/english/articles", label: "英语学习", icon: "languages" },
    ],
  },
  {
    label: "财务",
    items: [
      { route: "/finance", label: "财务中心", icon: "chart" },
    ],
  },
];

export const SECONDARY_NAV: NavItem[] = [
  { route: "/devices", label: "设备与会话", icon: "devices" },
  { route: "/settings", label: "数据与设置", icon: "settings" },
];

export const MOBILE_NAV: NavItem[] = [
  { route: "/", label: "总览", icon: "home" },
  { route: "/execution", label: "计划", icon: "check" },
  { route: "/finance", label: "财务", icon: "chart" },
  { route: "/notes", label: "笔记", icon: "note" },
  { route: "/settings", label: "设置", icon: "settings" },
];

export const ROUTES = new Set<Route>([
  "/", "/assistant", "/execution", "/execution/control", "/habits", "/english/articles", "/english/vocabulary",
  "/english/stats", "/fitness", "/notes", "/calendar", "/review", "/finance",
  "/finance/transactions", "/finance/accounts", "/finance/categories",
  "/finance/budgets", "/finance/import", "/finance/beecount", "/devices", "/settings", "/search",
]);

export const PAGE_COPY: Record<Route, [string, string]> = {
  "/": ["今日总览", "把今天真正需要关注的任务、坚持、训练、学习、财务和复盘集中在一个工作台。"],
  "/assistant": ["AI 管家", "基于当前云端记录生成摘要、趋势和可执行建议。"],
  "/execution": ["计划与待办", "从快速收集到今天执行，把任务、计划、备忘和完成历史放在同一个闭环里。"],
  "/execution/control": ["执行控制台", "管理等待事项、提醒、任务依赖与重复日历例外，不把外部依赖混进普通待办。"],
  "/habits": ["坚持项目", "管理长期项目，关注完成率、累计量与真实趋势。"],
  "/english/articles": ["英语学习", "阅读、总结、高亮、生词与长期能力成长。"],
  "/english/vocabulary": ["生词本", "集中复习阅读中积累的词汇。"],
  "/english/stats": ["英语统计", "查看阅读、总结与词汇积累。"],
  "/fitness": ["健身训练", "记录训练、动作、组数和训练笔记。"],
  "/notes": ["笔记", "记录想法、知识与复盘，并跨设备同步。"],
  "/calendar": ["生活日历", "坚持、训练、账单、英语、复盘和执行时间块都落在具体的一天里。"],
  "/review": ["每日复盘", "每天两分钟，看清今天并为明天留下重点。"],
  "/finance": ["财务中心", "从资产、收支、预算和账本四个维度看清自己的钱。"],
  "/finance/transactions": ["账单管理", "搜索、筛选、编辑并维护全部收支记录。"],
  "/finance/accounts": ["账户管理", "集中维护银行卡、电子钱包、投资账户和现金。"],
  "/finance/categories": ["收支分类", "维护可复用的收入和支出分类。"],
  "/finance/budgets": ["预算管理", "用月度预算控制消费节奏。"],
  "/finance/import": ["账单导入", "从微信、支付宝和通用 CSV 批量导入账单。"],
  "/finance/beecount": ["BeeCount 云账本", "在 LifeTrace 中查看 BeeCount iOS 与 Web 端同步的账本数据。"],
  "/devices": ["设备与会话", "管理登录设备、活动会话和账号安全。"],
  "/settings": ["数据与设置", "管理云端同步、界面偏好和账号设置。"],
  "/search": ["全局搜索", "搜索任务、计划、备忘、账单、笔记、训练、英语和坚持记录。"],
};

export function currentRoute(pathname = typeof window === "undefined" ? "/" : window.location.pathname): Route {
  const path = pathname.replace(/\/$/, "") || "/";
  return ROUTES.has(path as Route) ? path as Route : "/";
}

export function navigate(route: Route): void {
  if (window.location.pathname !== route) window.history.pushState({}, "", route);
  window.dispatchEvent(new PopStateEvent("popstate"));
}

export function routeIsActive(current: Route, target: Route): boolean {
  if (target === "/") return current === "/";
  if (target === "/execution") return current === "/execution" || current.startsWith("/execution/");
  if (target === "/finance") return current === "/finance" || current.startsWith("/finance/");
  if (target === "/english/articles") return current === "/english/articles" || current.startsWith("/english/");
  return current === target;
}
