import { useCallback, useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { Check, LoaderCircle, RefreshCw } from "lucide-react";
import { useAppUpdaterStore } from "@/src/stores/useAppUpdaterStore";
import { isTauriDesktopRuntime } from "@/src/services/appUpdater";

export default function AboutLifeTracePanel() {
  const state = useAppUpdaterStore((store) => store.state);
  const checking = useAppUpdaterStore((store) => store.checking);
  const update = useAppUpdaterStore((store) => store.update);
  const check = useAppUpdaterStore((store) => store.check);
  const download = useAppUpdaterStore((store) => store.download);
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    if (!isTauriDesktopRuntime()) return;
    getVersion()
      .then((value) => {
        if (!cancelled) setVersion(value);
      })
      .catch(() => {
        if (!cancelled) setVersion(null);
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
            应用版本 <b>{version ?? (isTauriDesktopRuntime() ? "未知" : "仅桌面版")}</b>
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
      </div>
    </article>
  );
}
