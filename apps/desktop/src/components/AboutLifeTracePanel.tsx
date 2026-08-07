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
  const blob = new Blob([JSON.stringify(value, null, 2)], { type: "application/json;charset=utf-8" });
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
    getVersion().then((value) => { if (!cancelled) setVersion(value); }).catch((error) => {
      clientLogger.warn("about.version.read_failed", undefined, error);
      if (!cancelled) setVersion(null);
    });
    getClientLogPath().then((value) => { if (!cancelled) setLogPath(value); }).catch(() => {
      if (!cancelled) setLogPath(null);
    });
    return () => { cancelled = true; };
  }, []);

  const busy = state.status === "downloading" || state.status === "installing";
  const statusText = useCallback(() => {
    switch (state.status) {
      case "checking": return "正在检查更新…";
      case "upToDate": return "当前已是最新版本";
      case "available": return update ? `发现新版本 v${update.version}` : "发现新版本";
      case "downloading": return `正在下载 v${state.version}…`;
      case "installing": return "正在安装更新，应用将自动重启";
      case "error": return state.message;
      default: return "通过 GitHub Releases 获取桌面版更新";
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
      clientLogger.info("diagnostics.export.succeeded", { eventCount: snapshot.recentEvents.length });
      setDiagnosticsMessage("脱敏诊断文件已导出");
    } catch (error) {
      clientLogger.error("diagnostics.export.failed", undefined, error);
      setDiagnosticsMessage(error instanceof Error ? error.message : "导出诊断信息失败");
    } finally {
      setDiagnosticsBusy(false);
    }
  };

  return <section id="settings-about" className="hx-settings-page-section hx-about-panel">
    <header><h2>关于</h2><p>版本、更新与客户端诊断。</p></header>
    <div className="hx-setting-rows">
      <div className="hx-setting-row"><div><FileText /><span><strong>LifeTrace</strong><small>本地优先的个人管理系统</small></span></div><span>{version ?? (isTauriDesktopRuntime() ? "未知版本" : "浏览器版")}</span></div>
      <div className="hx-setting-row"><div><RefreshCw /><span><strong>软件更新</strong><small>{statusText()}</small></span></div><div className="hx-settings-inline-actions"><button type="button" className="hx-btn secondary" disabled={checking || busy} onClick={() => void check("manual")}>{checking ? <LoaderCircle className="spin" /> : <RefreshCw />}检查更新</button>{state.status === "available" && update && <button type="button" className="hx-btn primary" disabled={busy} onClick={() => void download()}>下载并安装</button>}{state.status === "upToDate" && <span className="hx-setting-status"><Check />最新</span>}</div></div>
      <div className="hx-setting-row"><div><FileText /><span><strong>客户端日志</strong><small>{logPath || (isTauriDesktopRuntime() ? "本地日志已启用" : "浏览器诊断已启用")}</small></span></div><div className="hx-settings-inline-actions"><button type="button" className="hx-btn secondary" disabled={diagnosticsBusy} onClick={() => void copyDiagnostics()}>{diagnosticsBusy ? <LoaderCircle className="spin" /> : <Copy />}复制诊断</button><button type="button" className="hx-btn secondary" disabled={diagnosticsBusy} onClick={() => void exportDiagnostics()}><Download />导出</button></div></div>
    </div>
    {diagnosticsMessage && <p className="hx-inline-message" role="status">{diagnosticsMessage}</p>}
  </section>;
}
