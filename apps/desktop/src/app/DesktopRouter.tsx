import Dashboard from "@/src/components/feature/dashboard/Dashboard";
import Habits from "@/src/components/feature/habits/Habits";
import Fitness from "@/src/components/feature/fitness/Fitness";
import Finance from "@/src/components/feature/finance/Finance";
import Transactions from "@/src/components/feature/finance/Transactions";
import Accounts from "@/src/components/feature/finance/Accounts";
import ImportBills from "@/src/components/feature/finance/ImportBills";
import CalendarView from "@/src/components/feature/life/CalendarView";
import ReviewView from "@/src/components/feature/life/ReviewView";
import AnalyticsModule from "@/src/components/feature/analytics/AnalyticsModule";
import SettingsView from "@/src/components/feature/settings/SettingsView";
import DesignGallery from "@/src/components/design/DesignGallery";
import DailyEnglish from "@/src/components/english/DailyEnglish";
import ExecutionModule from "@/src/components/feature/execution/ExecutionModule";
import MailActionCenter from "@/src/components/feature/mail/MailActionCenter";
import NotesModule from "@/src/components/NotesModule";
import PhotoSyncModule from "@/src/components/PhotoSyncModule";
import AIAssistantModule from "@/src/components/AIAssistantModule";
import type { EditorModalState } from "@/src/components/feature/forms/EditorModal";
import type { PlatformView } from "@/src/components/layout/navigation";
import { noteApi } from "@/src/services/noteApi";
import { dayKey, escapeHtml } from "@/src/utils/format";

type DesktopRouterProps = {
  view: PlatformView;
  navigate(view: PlatformView): void;
  openEditor(modal: EditorModalState): void;
};

export default function DesktopRouter({ view, navigate, openEditor }: DesktopRouterProps) {
  const openNotes = (id?: string) => {
    if (id) window.localStorage.setItem("lifetrace:last-note", id);
    navigate("notes");
  };

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
    openNotes(created.id);
    window.dispatchEvent(
      new CustomEvent("hengxu-toast", { detail: "关联笔记已创建" }),
    );
  };

  const openAnalyticsEntity = (entityType: string, entityId: string) => {
    switch (entityType) {
      case "note":
        openNotes(entityId);
        return;
      case "transaction":
        navigate("transactions");
        return;
      case "habit":
      case "activity_log":
        navigate("habits");
        return;
      case "daily_review":
        navigate("review");
        return;
      case "workout":
        navigate("fitness");
        return;
      case "english_article":
      case "english_learning_record":
      case "vocabulary":
        navigate("english");
        return;
      case "calendar_event":
      case "execution_task":
      case "memo":
        navigate("execution");
        return;
      default:
        window.dispatchEvent(
          new CustomEvent("hengxu-toast", { detail: "已定位到记录所属模块" }),
        );
        navigate("dashboard");
    }
  };

  switch (view) {
    case "dashboard":
      return (
        <Dashboard
          go={(next) => navigate(next as PlatformView)}
          record={(value) => openEditor({ kind: "record", value })}
          openNotes={openNotes}
        />
      );
    case "execution":
      return <ExecutionModule />;
    case "assistant":
      return <AIAssistantModule openSettings={() => navigate("settings")} />;
    case "mail":
      return <MailActionCenter />;
    case "habits":
      return (
        <Habits
          edit={(value) => openEditor({ kind: "activity", value })}
          record={(value) => openEditor({ kind: "record", value })}
          note={(value) =>
            void makeLinkedNote(
              "habit_log",
              `${value.name}练习记录 - ${dayKey()}`,
              "habit",
              value.id,
              "今天的记录：\n\n问题：\n\n下次重点：",
            )
          }
        />
      );
    case "english":
      return <DailyEnglish />;
    case "fitness":
      return (
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
      );
    case "photos":
      return <PhotoSyncModule />;
    case "notes":
      return <NotesModule />;
    case "finance":
      return <Finance />;
    case "transactions":
      return (
        <Transactions
          edit={(value) => openEditor({ kind: "transaction", value })}
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
      );
    case "accounts":
      return <Accounts edit={(value) => openEditor({ kind: "account", value })} />;
    case "import":
      return <ImportBills />;
    case "calendar":
      return <CalendarView />;
    case "review":
      return <ReviewView />;
    case "analytics":
      return <AnalyticsModule openEntity={openAnalyticsEntity} />;
    case "settings":
      return <SettingsView />;
    case "gallery":
      return <DesignGallery />;
    default:
      return (
        <Dashboard
          go={(next) => navigate(next as PlatformView)}
          record={(value) => openEditor({ kind: "record", value })}
          openNotes={openNotes}
        />
      );
  }
}