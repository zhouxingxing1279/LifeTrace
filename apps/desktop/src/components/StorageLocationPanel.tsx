import { useCallback, useEffect, useState } from "react";
import { FolderCog, HardDrive, LoaderCircle, RotateCcw } from "lucide-react";

function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let current = value;
  let index = 0;
  while (current >= 1024 && index < units.length - 1) {
    current /= 1024;
    index += 1;
  }
  return `${current >= 10 || index === 0 ? current.toFixed(0) : current.toFixed(1)} ${units[index]}`;
}

function phaseText(status: StorageMigrationStatus): string {
  switch (status.phase) {
    case "copying":
      return `正在后台迁移 ${Math.round(status.progress)}%`;
    case "finalizing":
      return "正在保存迁移状态…";
    case "ready":
      return "后台复制完成，重启后完成最终切换";
    case "error":
      return status.error || "迁移失败";
    default:
      return "当前存储位置";
  }
}

export default function StorageLocationPanel() {
  const [status, setStatus] = useState<StorageMigrationStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const desktopAvailable = Boolean(window.storageApi);

  const refresh = useCallback(async () => {
    if (!window.storageApi) return;
    try {
      setStatus(await window.storageApi.status());
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!window.storageApi || !status || !["copying", "finalizing"].includes(status.phase)) return;
    const timer = window.setInterval(() => void refresh(), 600);
    return () => window.clearInterval(timer);
  }, [refresh, status?.phase]);

  const chooseLocation = async () => {
    if (!window.storageApi) return;
    setBusy(true);
    setMessage(null);
    try {
      const result = await window.storageApi.chooseAndMigrate();
      if (result.canceled) return;
      if (result.error) {
        setMessage(result.error);
        return;
      }
      if (result.status) setStatus(result.status);
      window.setTimeout(() => void refresh(), 300);
    } finally {
      setBusy(false);
    }
  };

  const restart = async () => {
    if (!window.storageApi) return;
    setBusy(true);
    setMessage(null);
    try {
      await window.storageApi.restart();
    } catch (error) {
      setBusy(false);
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  return <section id="settings-storage" className="hx-settings-page-section">
    <header>
      <h2>存储</h2>
      <p>管理 LifeTrace 本地数据的保存位置。</p>
    </header>
    <div className="hx-setting-rows">
      <div className="hx-setting-row">
        <div>
          <HardDrive />
          <span>
            <strong>本地数据位置</strong>
            <small>{status?.currentPath || (desktopAvailable ? "正在读取…" : "仅桌面版可更改")}</small>
          </span>
        </div>
        <button
          type="button"
          className="hx-btn secondary"
          disabled={!desktopAvailable || busy || status?.phase === "copying" || status?.phase === "finalizing" || status?.restartRequired}
          onClick={() => void chooseLocation()}
        >
          {busy && !status?.restartRequired ? <LoaderCircle className="spin" /> : <FolderCog />}
          更改位置
        </button>
      </div>

      {status?.targetPath && status.phase !== "idle" && <div className="hx-setting-row">
        <div>
          <FolderCog />
          <span>
            <strong>{phaseText(status)}</strong>
            <small>{status.targetPath}</small>
          </span>
        </div>
        {status.restartRequired && <button type="button" className="hx-btn primary" disabled={busy} onClick={() => void restart()}>
          {busy ? <LoaderCircle className="spin" /> : <RotateCcw />}
          重启并完成迁移
        </button>}
      </div>}
    </div>

    {status && ["copying", "finalizing"].includes(status.phase) && <div className="hx-storage-migration-progress" role="status" aria-live="polite">
      <div><span>{phaseText(status)}</span><strong>{Math.round(status.progress)}%</strong></div>
      <progress max={100} value={status.progress} />
      <small>{status.filesCopied} / {status.filesTotal} 个文件 · {formatBytes(status.bytesCopied)} / {formatBytes(status.bytesTotal)}</small>
    </div>}

    <p className="hx-settings-note">
      会迁移 SQLite 数据库、照片、附件、加密相册、备份、同步状态等本地内容。大文件复制在后台线程执行，迁移期间可以继续使用 LifeTrace；重启后会先做最终增量校准和完整性检查，确认成功后才删除旧目录。
    </p>
    {message && <p className="hx-inline-message" role="alert">{message}</p>}
  </section>;
}
