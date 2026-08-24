import { useEffect } from "react";
import { Check, Download, LoaderCircle, RefreshCw, X } from "lucide-react";
import { shouldAutoCheckForUpdate } from "@/src/services/appUpdater";
import { useAppUpdaterStore } from "@/src/stores/useAppUpdaterStore";

let autoCheckStarted = false;

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDate(value: string | null): string {
  if (!value) return "未知";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("zh-CN", { dateStyle: "medium", timeStyle: "short" });
}

export default function AppUpdaterHost() {
  const state = useAppUpdaterStore((store) => store.state);
  const update = useAppUpdaterStore((store) => store.update);
  const check = useAppUpdaterStore((store) => store.check);
  const download = useAppUpdaterStore((store) => store.download);
  const dismiss = useAppUpdaterStore((store) => store.dismiss);

  useEffect(() => {
    if (autoCheckStarted) return;
    autoCheckStarted = true;
    if (!shouldAutoCheckForUpdate()) return;
    const timer = window.setTimeout(() => {
      void check("auto").catch(() => undefined);
    }, 4000);
    return () => window.clearTimeout(timer);
  }, [check]);

  const visible =
    state.status === "available" ||
    state.status === "downloading" ||
    state.status === "installing" ||
    state.status === "error";
  if (!visible) return null;

  const busy =
    state.status === "downloading" || state.status === "installing";
  const canClose = !busy;

  return (
    <div className="hx-overlay" role="dialog" aria-modal="true" aria-label="应用更新">
      <div className="hx-modal hx-update-modal">
        <header>
          <div>
            <span className="hx-kicker">UPDATER</span>
            <h2>应用更新</h2>
          </div>
          {canClose && (
            <button type="button" aria-label="关闭更新弹窗" onClick={dismiss}>
              <X />
            </button>
          )}
        </header>
        <div className="hx-update-body">
          {state.status === "available" && update && (
            <>
              <div className="hx-update-versions">
                <span>
                  <small>当前版本</small>
                  <b>v{update.currentVersion}</b>
                </span>
                <i>→</i>
                <span className="new">
                  <small>新版本</small>
                  <b>v{update.version}</b>
                </span>
              </div>
              {update.notes && (
                <div className="hx-update-notes">
                  <small>更新说明</small>
                  <p>{update.notes}</p>
                </div>
              )}
              {update.date && (
                <p className="hx-update-date">发布时间：{formatDate(update.date)}</p>
              )}
              <div className="hx-update-actions">
                <button
                  type="button"
                  className="hx-btn primary"
                  onClick={() => void download()}
                >
                  <Download /> 下载并安装
                </button>
                <button
                  type="button"
                  className="hx-btn secondary"
                  onClick={dismiss}
                >
                  稍后提醒
                </button>
              </div>
            </>
          )}

          {state.status === "downloading" && (
            <div className="hx-update-download">
              <div className="hx-update-status">
                <LoaderCircle className="spin" />
                <strong>正在下载更新 v{state.version}…</strong>
              </div>
              <div className="hx-update-track">
                <i>
                  <b
                    className={state.percentage === null ? "indeterminate" : undefined}
                    style={
                      state.percentage === null
                        ? undefined
                        : { width: `${state.percentage}%` }
                    }
                  />
                </i>
              </div>
              <p>
                {state.percentage === null
                  ? `已下载 ${formatBytes(state.downloadedBytes)}，正在获取剩余大小…`
                  : `已下载 ${formatBytes(state.downloadedBytes)} / ${
                      state.totalBytes === null
                        ? "未知"
                        : formatBytes(state.totalBytes)
                    }（${state.percentage}%）`}
              </p>
            </div>
          )}

          {state.status === "installing" && (
            <div className="hx-update-status">
              <LoaderCircle className="spin" />
              <strong>正在安装更新，应用将自动重启。</strong>
            </div>
          )}

          {state.status === "error" && (
            <>
              <div className="hx-update-error" role="alert">
                <strong>更新失败</strong>
                <p>{state.message}</p>
              </div>
              <div className="hx-update-actions">
                <button
                  type="button"
                  className="hx-btn primary"
                  onClick={() => void check("manual")}
                >
                  <RefreshCw /> 重试
                </button>
                {update && (
                  <button
                    type="button"
                    className="hx-btn primary"
                    onClick={() => void download()}
                  >
                    <Download /> 继续下载
                  </button>
                )}
                <button
                  type="button"
                  className="hx-btn secondary"
                  onClick={dismiss}
                >
                  <Check /> 关闭
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
