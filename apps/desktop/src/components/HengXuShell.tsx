"use client";

import { useEffect, useState } from "react";
import {
  Check,
  Home,
  Moon,
  Palette,
  Plus,
  Settings,
  Sun,
} from "lucide-react";
import DesktopRouter from "@/src/app/DesktopRouter";
import { useLifeStore } from "@/src/stores/useLifeStore";
import AppShell from "@/src/components/layout/AppShell";
import type { CommandItem } from "@/src/components/layout/CommandPalette";
import {
  isPlatformView,
  navGroups,
  pageTitles,
  type PlatformView,
} from "@/src/components/layout/navigation";
import EditorModal, {
  type EditorModalState,
} from "@/src/components/feature/forms/EditorModal";
import { ConfirmDialogHost } from "@/src/ui/feedback/confirm";
import AppUpdaterHost from "@/src/components/AppUpdaterHost";
import type { ToastPayload } from "@/src/ui/feedback/toastBus";

const DENSITY_KEY = "lifetrace:ui-density";

export default function HengXuShell() {
  const { ready, storageError, initialize } = useLifeStore();
  const [view, setView] = useState<PlatformView>(() => {
    if (typeof window === "undefined") return "dashboard";
    const requested = new URLSearchParams(window.location.search).get("view");
    return requested && isPlatformView(requested)
      ? requested
      : "dashboard";
  });
  const [modal, setModal] = useState<EditorModalState>(null);
  const [toast, setToast] = useState("");
  const [toastDuration, setToastDuration] = useState(2200);

  useEffect(() => {
    void initialize();
  }, [initialize]);

  useEffect(() => {
    const stored = window.localStorage.getItem(DENSITY_KEY);
    document.documentElement.dataset.density =
      stored === "compact" ? "compact" : "comfortable";
  }, []);

  useEffect(() => {
    const receive = (event: Event) => {
      const detail = (
        event as CustomEvent<string | Partial<ToastPayload>>
      ).detail;
      if (typeof detail === "string") {
        setToastDuration(2200);
        setToast(detail);
      } else if (detail?.message) {
        setToastDuration(detail.duration ?? (detail.type === "error" ? 4500 : 2500));
        setToast(detail.message);
      }
    };
    window.addEventListener("hengxu-toast", receive);
    return () => window.removeEventListener("hengxu-toast", receive);
  }, []);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(""), toastDuration);
    const element = document.querySelector(".hx-toast");
    const close = () => setToast("");
    element?.addEventListener("click", close);
    return () => {
      window.clearTimeout(timer);
      element?.removeEventListener("click", close);
    };
  }, [toast, toastDuration]);

  if (!ready) {
    return (
      <div className="hx-loading">
        <span>LT</span>
        <p>正在连接 SQLite 个人系统…</p>
      </div>
    );
  }

  if (storageError) {
    return (
      <div className="hx-loading">
        <span>!</span>
        <h1>SQLite 暂时无法连接</h1>
        <p>{storageError}</p>
        <button type="button" className="hx-btn primary" onClick={() => initialize()}>
          重新连接
        </button>
      </div>
    );
  }

  const toggleDensity = () => {
    const next =
      document.documentElement.dataset.density === "compact"
        ? "comfortable"
        : "compact";
    document.documentElement.dataset.density = next;
    window.localStorage.setItem(DENSITY_KEY, next);
    window.dispatchEvent(
      new CustomEvent("hengxu-toast", {
        detail: next === "compact" ? "已切换紧凑布局" : "已切换舒适布局",
      }),
    );
  };

  const commandItems: CommandItem[] = [
    {
      id: "nav-dashboard",
      label: "前往总览",
      hint: "今日坚持与最近动态",
      icon: Home,
      group: "跳转",
      execute: () => setView("dashboard"),
    },
    ...navGroups.flatMap((group) =>
      group.items.map((item) => ({
        id: `nav-${item.id}`,
        label: `前往${item.label}`,
        hint: group.label,
        icon: item.icon,
        group: "跳转",
        execute: () => setView(item.id as PlatformView),
      })),
    ),
    {
      id: "nav-settings",
      label: "打开设置",
      hint: "数据、备份与应用设置",
      icon: Settings,
      group: "操作",
      execute: () => setView("settings"),
    },
    {
      id: "new-habit",
      label: "新建坚持项目",
      icon: Plus,
      group: "新建",
      execute: () => {
        setView("habits");
        setModal({ kind: "activity" });
      },
    },
    {
      id: "new-transaction",
      label: "手动记账",
      icon: Plus,
      group: "新建",
      execute: () => {
        setView("transactions");
        setModal({ kind: "transaction" });
      },
    },
    {
      id: "new-account",
      label: "添加账户",
      icon: Plus,
      group: "新建",
      execute: () => {
        setView("accounts");
        setModal({ kind: "account" });
      },
    },
    {
      id: "toggle-density",
      label:
        document.documentElement.dataset.density === "compact"
          ? "切换舒适布局"
          : "切换紧凑布局",
      icon: Palette,
      group: "操作",
      execute: toggleDensity,
    },
    {
      id: "toggle-theme",
      label: useLifeStore.getState().dark ? "切换到浅色主题" : "切换到深色主题",
      icon: useLifeStore.getState().dark ? Sun : Moon,
      group: "操作",
      execute: () => useLifeStore.getState().toggleDark(),
    },
  ];

  return (
    <>
      <AppShell
        view={view}
        navGroups={navGroups}
        title={pageTitles[view]}
        onNavigate={(next) => {
          if (isPlatformView(next)) setView(next);
        }}
        commandItems={commandItems}
      >
        <DesktopRouter
          view={view}
          navigate={setView}
          openEditor={setModal}
        />
      </AppShell>

      {modal ? <EditorModal modal={modal} close={() => setModal(null)} /> : null}
      {toast ? (
        <div className="hx-toast" role="status">
          <Check aria-hidden="true" />
          {toast}
        </div>
      ) : null}
      <ConfirmDialogHost />
      <AppUpdaterHost />
    </>
  );
}
