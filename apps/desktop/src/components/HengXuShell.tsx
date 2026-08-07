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
import { useLifeStore } from "@/src/stores/useLifeStore";
import { noteApi } from "@/src/services/noteApi";
import AppShell from "@/src/components/layout/AppShell";
import type { CommandItem } from "@/src/components/layout/CommandPalette";
import {
  isPlatformView,
  navGroups,
  pageTitles,
  type PlatformView,
} from "@/src/components/layout/navigation";
import Dashboard from "@/src/components/feature/dashboard/Dashboard";
import Habits from "@/src/components/feature/habits/Habits";
import Fitness from "@/src/components/feature/fitness/Fitness";
import Finance from "@/src/components/feature/finance/Finance";
import Transactions from "@/src/components/feature/finance/Transactions";
import Accounts from "@/src/components/feature/finance/Accounts";
import ImportBills from "@/src/components/feature/finance/ImportBills";
import CalendarView from "@/src/components/feature/life/CalendarView";
import ReviewView from "@/src/components/feature/life/ReviewView";
import SettingsView from "@/src/components/feature/settings/SettingsView";
import DesignGallery from "@/src/components/design/DesignGallery";
import EditorModal, {
  type EditorModalState,
} from "@/src/components/feature/forms/EditorModal";
import DailyEnglish from "@/src/components/english/DailyEnglish";
import NotesModule from "@/src/components/NotesModule";
import PhotoSyncModule from "@/src/components/PhotoSyncModule";
import AIAssistantModule from "@/src/components/AIAssistantModule";
import { ConfirmDialogHost } from "@/src/ui/feedback/confirm";
import AppUpdaterHost from "@/src/components/AppUpdaterHost";
import type { ToastPayload } from "@/src/ui/feedback/toastBus";
import { escapeHtml, dayKey } from "@/src/utils/format";

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
  const [menuOpen, setMenuOpen] = useState(false);
  const [toast, setToast] = useState("");
  const [toastDuration, setToastDuration] = useState(2200);

  const makeLinkedNote = async (
    noteType: "habit_log" | "workout_review" | "expense_note",
    title: string,
    entityType: "habit" | "workout" | "transaction",
    entityId: string,
    content: string,
  ) => {
    const created = await noteApi.create({
      title,
      noteType,
      folderId: null,
      contentJson: {
        type: "doc",
        content: [
          {
            type: "paragraph",
            content: [{ type: "text", text: content }],
          },
        ],
      },
      contentHtml: `<p>${escapeHtml(content).replace(/\n/g, "<br>")}</p>`,
      contentText: content,
      contentMarkdown: content,
      summary: content.replace(/\s+/g, " ").slice(0, 160),
      isPinned: false,
      isFavorite: false,
      isArchived: false,
      tagIds: [],
      relations: [
        {
          id: crypto.randomUUID(),
          noteId: "pending",
          entityType,
          entityId,
          relationType: "created_from",
          createdAt: new Date().toISOString(),
        },
      ],
    });
    window.localStorage.setItem("lifetrace:last-note", created.id);
    setView("notes");
    window.dispatchEvent(
      new CustomEvent("hengxu-toast", { detail: "关联笔记已创建" }),
    );
  };

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
          setMenuOpen(false);
        }}
        commandItems={commandItems}
      >
        {view === "dashboard" ? (
          <Dashboard
            go={(next) => setView(next as PlatformView)}
            record={(value) => setModal({ kind: "record", value })}
            openNotes={(id) => {
              if (id) window.localStorage.setItem("lifetrace:last-note", id);
              setView("notes");
            }}
          />
        ) : null}
        {view === "assistant" ? (
          <AIAssistantModule openSettings={() => setView("settings")} />
        ) : null}
        {view === "habits" ? (
          <Habits
            edit={(value) => setModal({ kind: "activity", value })}
            record={(value) => setModal({ kind: "record", value })}
            note={(value) =>
              void makeLinkedNote(
                "habit_log",
                `${value.name}练习记录 - ${dayKey()}`,
                "habit",
                value.id,
                `今天的记录：\n\n问题：\n\n下次重点：`,
              )
            }
          />
        ) : null}
        {view === "english" ? <DailyEnglish /> : null}
        {view === "fitness" ? (
          <Fitness
            note={(value) =>
              void makeLinkedNote(
                "workout_review",
                `训练复盘 - ${dayKey(new Date(value.occurredAt))}`,
                "workout",
                value.id,
                `训练名称：${value.name}\n训练日期：${dayKey(new Date(value.occurredAt))}\n训练时长：${Math.max(1, Math.round(value.durationSeconds / 60))} 分钟\n总容量：${value.volumeKg ?? "未记录"}\n动作数量：${value.exerciseCount}\n训练来源：${value.source}`,
              )
            }
          />
        ) : null}
        {view === "photos" ? <PhotoSyncModule /> : null}
        {view === "notes" ? <NotesModule /> : null}
        {view === "finance" ? <Finance /> : null}
        {view === "transactions" ? (
          <Transactions
            edit={(value) => setModal({ kind: "transaction", value })}
            note={(value) =>
              void makeLinkedNote(
                "expense_note",
                `消费记录 - ${value.counterparty || value.category}`,
                "transaction",
                value.id,
                `日期：${dayKey(new Date(value.occurredAt))}\n金额：¥${value.amount.toFixed(2)}\n分类：${value.category}\n账户：${value.account}\n商户：${value.counterparty || "未填写"}\n消费目的：`,
              )
            }
          />
        ) : null}
        {view === "accounts" ? (
          <Accounts
            edit={(value) => setModal({ kind: "account", value })}
          />
        ) : null}
        {view === "import" ? <ImportBills /> : null}
        {view === "calendar" ? <CalendarView /> : null}
        {view === "review" ? <ReviewView /> : null}
        {view === "settings" ? <SettingsView /> : null}
        {view === "gallery" ? <DesignGallery /> : null}
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
