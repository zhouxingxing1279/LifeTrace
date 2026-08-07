import { useRef, useState } from "react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import { noteApi } from "@/src/services/noteApi";
import AppearanceSettingsPanel from "@/src/components/AppearanceSettingsPanel";
import CloudAccountPanel from "@/src/components/CloudAccountPanel";
import AISettingsPanel from "@/src/components/AISettingsPanel";
import TranslationSettingsPanel from "@/src/components/TranslationSettingsPanel";
import AboutLifeTracePanel from "@/src/components/AboutLifeTracePanel";
import { PanelHead } from "@/src/components/common";

export default function SettingsView() {
  const store = useLifeStore();
  const input = useRef<HTMLInputElement>(null);
  const [message, setMessage] = useState("");
  const download = (
    text: string,
    name: string,
    type = "application/json",
  ) => {
    const url = URL.createObjectURL(new Blob([text], { type }));
    const link = document.createElement("a");
    link.href = url;
    link.download = name;
    link.click();
    URL.revokeObjectURL(url);
  };
  const backup = {
    format: "lifetrace-backup",
    schemaVersion: 2,
    createdAt: new Date().toISOString(),
    activities: store.activities,
    logs: store.logs,
    transactions: store.transactions,
    reviews: store.reviews,
    accounts: store.accounts,
    workoutHistory: store.workoutHistory,
  };

  return (
    <div className="hx-view">
      <div className="hx-settings-grid">
        <AppearanceSettingsPanel />
        <CloudAccountPanel />
        <AISettingsPanel />
        <TranslationSettingsPanel />
        <AboutLifeTracePanel />
        <article className="hx-panel">
          <PanelHead kicker="数据备份" title="数据备份" />
          <div className="hx-panel-body">
            <p>
              导出完整 JSON 备份，包含坚持、复盘、训练、账户、账单、笔记、标签、关联和版本历史。
            </p>
            <div className="hx-settings-actions">
              <button
                type="button"
                className="hx-btn primary"
                onClick={async () => {
                  try {
                    const notesBackup = await noteApi.backup();
                    download(
                      JSON.stringify({ ...backup, notesBackup }, null, 2),
                      "life-trace-backup.json",
                    );
                  } catch (error) {
                    setMessage(error instanceof Error ? error.message : "导出失败");
                  }
                }}
              >
                导出备份
              </button>
              <button
                type="button"
                className="hx-btn secondary"
                onClick={() => input.current?.click()}
              >
                恢复备份
              </button>
              <input
                ref={input}
                hidden
                type="file"
                accept=".json,application/json"
                onChange={async (event) => {
                  const file = event.target.files?.[0];
                  if (!file) return;
                  try {
                    const data = JSON.parse(
                      await file.text(),
                    ) as Record<string, unknown>;
                    await store.restoreBackup(data);
                    if (data.notesBackup)
                      await noteApi.restoreBackup(
                        data.notesBackup as Record<string, unknown>,
                      );
                    setMessage("完整备份已恢复到 SQLite");
                  } catch (error) {
                    setMessage(error instanceof Error ? error.message : "恢复失败");
                  }
                }}
              />
            </div>
            {message ? (
              <p className="hx-inline-message">{message}</p>
            ) : null}
          </div>
        </article>
        <article className="hx-panel">
          <PanelHead kicker="本地存储" title="SQLite 存储状态" />
          <div className="hx-panel-body hx-storage">
            <span>
              坚持项目 <b>{store.activities.length} 个</b>
            </span>
            <span>
              坚持记录 <b>{store.logs.length} 条</b>
            </span>
            <span>
              训练历史 <b>{store.workoutHistory.length} 条</b>
            </span>
            <span>
              账户 / 账单 <b>{store.accounts.length} / {store.transactions.length}</b>
            </span>
            <span>
              笔记数据库 <b className="positive">已纳入备份</b>
            </span>
            <span>
              数据库连接 <b className="positive">正常</b>
            </span>
          </div>
        </article>
      </div>
    </div>
  );
}
