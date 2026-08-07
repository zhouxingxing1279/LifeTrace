import { useCallback, useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { Check, Copy, Download, FileText, LoaderCircle, RefreshCw } from "lucide-react";
import { useAppUpdaterStore } from "@/src/stores/useAppUpdaterStore";
import { isTauriDesktopRuntime } from "@/src/services/appUpdater";
import {
  copyClientDiagnostics,
  createClientDiagnosticSnapshot,
  getClientLogPath,
} from "@/src/services/clientDiagnostics";
import { clientLogger } from "@/src/services/clientObservability";

function downloadJson(value: unknown, filename: string): void {
  const blob = new Blob([JSON.stringify(value, null, 2)], {
    type: "application/json;charset=utf-8",
  });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}

export default function AboutLifeTracePanel() {
  const state = useAppUpdaterStore((store) => store.state);
  const checking = useAppUpdaterStore((store) => store.checking);
  const update = useAppUpdaterStore((store) => store.update);
  const check = useAppUpdaterStore((store) => store.check);
  const download = useAppUpdaterStore((store) => store.download);
  const [version, setVersion] = useState<string | null>(null);
  const [logPath, setLogPath] = useState<string | null>(null);
  const [diagnosticsBusy, setDiagnosticsBusy] = useState(false);
  const [diagnosticsMessage, setDiagnosticsMessage] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    if (!isTauriDesktopRuntime()) return;
    getVersion()
      .then((value) => {
        if (!cancelled) setVersion(value);
      })
      .catch((error) => {
        clientLogger.warn("about.version.read_failed", undefined, error);
        if (!cancelled) setVersion(null);
      });
    getClientLogPath()
      .then((value) => {
        if (!cancelled) setLogPath(value);
      })
      .catch(() => {
        if (!cancelled) setLogPath(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const busy =
    state.status === "downloading" || state.status === "installing";
  const statusText = useCallback(() => {
    switch (state.status) {
      case "checking":
        return "正在检查更新…";
      case "upToDate":
        return "当前已是最新版本";
      case "available":
        return update
          ? `发现新版本 v${update.version}，可在弹窗中下载安装`
          : "发现新版本";
      case "downloading":
        return `正在下载 v${state.version}…`;
      case "installing":
        return "正在安装更新，应用将自动重启";
      case "error":
        return state.message;
      default:
        return null;
    }
  }, [state, update]);

  const copyDiagnostics = async () => {
    setDiagnosticsBusy(true);
    setDiagnosticsMessage(null);
    try {
      const snapshot = await copyClientDiagnostics();
      setDiagnosticsMessage(`已复制 ${snapshot.recentEvents.length} 条脱敏诊断记录`);
    } catch (error) {
      clientLogger.error("diagnostics.copy.failed", undefined, error);
      setDiagnosticsMessage(error instanceof Error ? error.message : "复制诊断信息失败");
    } finally {
      setDiagnosticsBusy(false);
    }
  };

  const exportDiagnostics = async () => {
    setDiagnosticsBusy(true);
    setDiagnosticsMessage(null);
    try {
      const snapshot = await createClientDiagnosticSnapshot();
      const stamp = new Date().toISOString().replace(/[:.]/g, "-");
      downloadJson(snapshot, `LifeTrace-Diagnostics-${stamp}.json`);
      clientLogger.info("diagnostics.export.succeeded", {
        eventCount: snapshot.recentEvents.length,
      });
      setDiagnosticsMessage("脱敏诊断文件已导出");
    } catch (error) {
      clientLogger.error("diagnostics.export.failed", undefined, error);
      setDiagnosticsMessage(error instanceof Error ? error.message : "导出诊断信息失败");
    } finally {
      setDiagnosticsBusy(false);
    }
  };

  return (
    <article className="hx-panel hx-about-panel">
      <div className="hx-panel-head">
        <div>
          <span className="hx-kicker">ABOUT</span>
          <h2>关于 LifeTrace</h2>
        </div>
      </div>
      <div className="hx-panel-body">
        <div className="hx-about-versions">
          <span>
            应用版本 <b>{version ?? (isTauriDesktopRuntime() ? "未知" : "浏览器版")}</b>
          </span>
          {update && (
            <span>
              可用更新 <b className="positive">v{update.version}</b>
            </span>
          )}
        </div>
        <p>
          LifeTrace 是一款本地优先的个人管理系统。桌面版通过 GitHub Releases
          提供在线更新，更新包经过签名校验后自动安装。
        </p>
        {statusText() && (
          <p className={`hx-about-status ${state.status === "error" ? "error" : ""}`}>
            {statusText()}
          </p>
        )}
        <div className="hx-settings-actions">
          <button
            type="button"
            className="hx-btn primary"
            disabled={checking || busy}
            onClick={() => void check("manual")}
          >
            {checking ? <LoaderCircle className="spin" /> : <RefreshCw />}
            检查更新
          </button>
          {state.status === "available" && update && (
            <button
              type="button"
              className="hx-btn secondary"
              disabled={busy}
              onClick={() => void download()}
            >
              下载并安装
            </button>
          )}
          {state.status === "upToDate" && (
            <span className="hx-about-latest">
              <Check /> 当前已是最新版本
            </span>
          )}
        </div>

        <div className="hx-panel-head">
          <div>
            <span className="hx-kicker">DIAGNOSTICS</span>
            <h2>错误诊断</h2>
          </div>
        </div>
        <p>
          登录、同步或界面异常会保留原始错误链和请求阶段。导出的诊断文件会自动隐藏密码、Token、Cookie 和授权头。
        </p>
        {logPath && (
          <div className="hx-about-versions">
            <span title={logPath}>
              日志文件 <b>{logPath}</b>
            </span>
          </div>
        )}
        <div className="hx-settings-actions">
          <button
            type="button"
            className="hx-btn secondary"
            disabled={diagnosticsBusy}
            onClick={() => void copyDiagnostics()}
          >
            {diagnosticsBusy ? <LoaderCircle className="spin" /> : <Copy />}
            复制诊断信息
          </button>
          <button
            type="button"
            className="hx-btn secondary"
            disabled={diagnosticsBusy}
            onClick={() => void exportDiagnostics()}
          >
            <Download />
            导出诊断文件
          </button>
          <span className="hx-about-latest">
            {isTauriDesktopRuntime() ? <FileText /> : <Check />}
            {isTauriDesktopRuntime() ? "本地日志已启用" : "浏览器诊断已启用"}
          </span>
        </div>
        {diagnosticsMessage && <p className="hx-about-status">{diagnosticsMessage}</p>}
      </div>
    </article>
  );
}
