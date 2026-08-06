"use client";

import { useEffect, useState } from "react";
import { Activity, Bot, Check, KeyRound, Monitor, RotateCcw, Trash2, Type } from "lucide-react";
import {
  applyAppPreferences,
  DEFAULT_APP_PREFERENCES,
  readAppPreferences,
  type AppPreferences,
  writeAppPreferences,
} from "@/src/services/appPreferences";

type AISettingsResponse = {
  provider: "deepseek";
  model: "deepseek-v4-flash" | "deepseek-v4-pro";
  configured: boolean;
  updatedAt?: string;
  error?: string;
};

const settingsLayoutStyles = `
  .hx-settings-grid {
    display:flex!important;flex-direction:column;gap:0!important;max-width:980px;margin:0 auto;
    overflow:hidden;background:var(--hx-panel);border:1px solid var(--hx-line);
    border-radius:18px;box-shadow:var(--hx-shadow);
  }
  .hx-settings-grid > .hx-settings-section,
  .hx-settings-grid > .hx-panel {
    margin:0!important;border:0!important;border-radius:0!important;box-shadow:none!important;
    background:transparent!important;overflow:visible!important;
  }
  .hx-settings-grid > .hx-settings-section + .hx-settings-section,
  .hx-settings-grid > .hx-panel { border-top:1px solid var(--hx-line)!important; }
  .hx-settings-grid > .hx-panel .hx-panel-head {
    min-height:auto;padding:24px 28px 14px;border-bottom:0;
  }
  .hx-settings-grid > .hx-panel .hx-panel-body { padding:0 28px 26px; }
  .hx-settings-basic{order:0}.hx-cloud-account{order:1}.hx-ai-settings{order:2}
  .hx-translation-settings{order:3}.hx-settings-grid > .hx-panel{order:4}
  .hx-settings-section-head {
    display:flex;align-items:flex-start;justify-content:space-between;gap:20px;padding:24px 28px 14px;
  }
  .hx-settings-section-head > div > span {
    display:block;margin-bottom:5px;color:var(--hx-muted);font-size:10px;letter-spacing:.13em;text-transform:uppercase;
  }
  .hx-settings-section-head h2 { margin:0;font-size:17px; }
  .hx-settings-section-head > i {
    width:38px;height:38px;display:grid;place-items:center;border-radius:12px;
    background:var(--hx-soft);color:var(--hx-muted);font-style:normal;
  }
  .hx-settings-section-head > i.configured { background:var(--hx-accent-soft);color:var(--hx-accent2); }
  .hx-settings-section-head svg { width:18px; }
  .hx-settings-section-body { padding:0 28px 26px; }
  .hx-settings-section-description { margin:0 0 18px;color:var(--hx-muted);font-size:11px;line-height:1.7; }
  .hx-settings-preference-list { border-top:1px solid var(--hx-line); }
  .hx-settings-preference-row {
    min-height:68px;display:grid;grid-template-columns:minmax(240px,1fr) minmax(190px,260px);
    align-items:center;gap:24px;border-bottom:1px solid var(--hx-line);padding:12px 0;
  }
  .hx-settings-preference-row > div { display:grid;grid-template-columns:32px 1fr;column-gap:10px;align-items:center; }
  .hx-settings-preference-row > div > svg { grid-row:1 / span 2;width:17px;color:var(--hx-accent); }
  .hx-settings-preference-row strong { font-size:12px; }
  .hx-settings-preference-row small { color:var(--hx-muted);font-size:9px;line-height:1.55; }
  .hx-settings-preference-row select { width:100%;height:39px;border:1px solid var(--hx-line);border-radius:10px;background:var(--hx-paper);padding:0 11px;color:var(--hx-ink); }
  .hx-settings-switch { justify-self:end;display:inline-flex;align-items:center;gap:9px;color:var(--hx-muted);font-size:10px; }
  .hx-settings-switch input { width:17px;height:17px;accent-color:var(--hx-accent); }
  .hx-settings-section-actions { display:flex;justify-content:flex-end;gap:9px;margin-top:17px; }
  .hx-settings-form { display:grid;gap:15px; }
  .hx-settings-form > label { display:grid;gap:7px;color:var(--hx-muted);font-size:10px;font-weight:650; }
  .hx-settings-form > label > input,.hx-settings-form > label > select {
    width:100%;height:42px;border:1px solid var(--hx-line);border-radius:11px;
    background:var(--hx-paper);color:var(--hx-ink);padding:0 12px;
  }
  .hx-settings-form > small { color:var(--hx-muted);font-size:9px;line-height:1.7; }
  .hx-settings-form > footer { display:flex;justify-content:flex-end;gap:9px;margin-top:2px; }
  .hx-settings-form .hx-secret-input { position:relative; }
  .hx-settings-form .hx-secret-input > svg { position:absolute;left:12px;top:12px;width:17px;color:var(--hx-muted); }
  .hx-settings-form .hx-secret-input > input {
    width:100%;height:42px;border:1px solid var(--hx-line);border-radius:11px;
    background:var(--hx-paper);color:var(--hx-ink);padding:0 12px 0 39px;
  }
  @media (max-width:760px) {
    .hx-settings-grid{border-radius:14px}.hx-settings-section-head{padding:20px 18px 12px}
    .hx-settings-section-body{padding:0 18px 22px}.hx-settings-grid > .hx-panel .hx-panel-head{padding:20px 18px 12px}
    .hx-settings-grid > .hx-panel .hx-panel-body{padding:0 18px 22px}
    .hx-settings-preference-row{grid-template-columns:1fr;gap:10px}.hx-settings-switch{justify-self:start}
  }
`;

export default function AISettingsPanel() {
  const [preferences, setPreferences] = useState<AppPreferences>(() => readAppPreferences());
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState<AISettingsResponse["model"]>("deepseek-v4-flash");
  const [configured, setConfigured] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    const controller = new AbortController();
    void fetch("/api/settings/ai", { signal: controller.signal })
      .then(async (response) => {
        const payload = await response.json() as AISettingsResponse;
        if (!response.ok) throw new Error(payload.error || "无法读取 AI 管家设置");
        setModel(payload.model);
        setConfigured(payload.configured);
      })
      .catch((error) => {
        if (!controller.signal.aborted) setMessage(error instanceof Error ? error.message : "无法读取 AI 管家设置");
      })
      .finally(() => setLoading(false));
    return () => controller.abort();
  }, []);

  const updatePreferences = (patch: Partial<AppPreferences>) => {
    setPreferences((current) => {
      const next = { ...current, ...patch };
      writeAppPreferences(next);
      applyAppPreferences(next);
      return next;
    });
  };

  const resetPreferences = () => {
    const next = { ...DEFAULT_APP_PREFERENCES };
    writeAppPreferences(next);
    applyAppPreferences(next);
    setPreferences(next);
  };

  const save = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!apiKey.trim() && !configured) {
      setMessage("请填写 DeepSeek API Key");
      return;
    }
    setSaving(true);
    setMessage("");
    try {
      const response = await fetch("/api/settings/ai", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ apiKey: apiKey || undefined, model }),
      });
      const payload = await response.json() as AISettingsResponse;
      if (!response.ok) throw new Error(payload.error || "AI 管家设置保存失败");
      setConfigured(payload.configured);
      setModel(payload.model);
      setApiKey("");
      setMessage("DeepSeek 配置已保存到本机");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "AI 管家设置保存失败");
    } finally {
      setSaving(false);
    }
  };

  const clear = async () => {
    if (!window.confirm("移除本机保存的 DeepSeek API Key？移除后 AI 管家将停止工作。")) return;
    setSaving(true);
    setMessage("");
    try {
      const response = await fetch("/api/settings/ai", { method: "DELETE" });
      if (!response.ok) throw new Error("DeepSeek 配置清除失败");
      setApiKey("");
      setConfigured(false);
      setMessage("DeepSeek 配置已移除");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "DeepSeek 配置清除失败");
    } finally {
      setSaving(false);
    }
  };

  return <>
    <style>{settingsLayoutStyles}</style>
    <section className="hx-settings-section hx-settings-basic">
      <header className="hx-settings-section-head">
        <div><span>应用偏好</span><h2>基础设置</h2></div>
        <i><Monitor /></i>
      </header>
      <div className="hx-settings-section-body">
        <p className="hx-settings-section-description">这些偏好只影响当前设备的界面，不会写入业务数据，也不会与 API Key 混合存储。</p>
        <div className="hx-settings-preference-list">
          <div className="hx-settings-preference-row">
            <div><Monitor /><strong>外观主题</strong><small>可固定浅色、深色，或跟随操作系统。</small></div>
            <select value={preferences.theme} onChange={(event) => updatePreferences({ theme: event.target.value as AppPreferences["theme"] })}>
              <option value="system">跟随系统</option><option value="light">浅色</option><option value="dark">深色</option>
            </select>
          </div>
          <div className="hx-settings-preference-row">
            <div><Activity /><strong>界面密度</strong><small>紧凑模式会减少列表、面板与工具栏留白。</small></div>
            <select value={preferences.density} onChange={(event) => updatePreferences({ density: event.target.value as AppPreferences["density"] })}>
              <option value="comfortable">舒适</option><option value="compact">紧凑</option>
            </select>
          </div>
          <div className="hx-settings-preference-row">
            <div><Type /><strong>界面字号</strong><small>统一调整应用基础字号，适用于高分屏或远距离使用。</small></div>
            <select value={preferences.fontScale} onChange={(event) => updatePreferences({ fontScale: event.target.value as AppPreferences["fontScale"] })}>
              <option value="small">小</option><option value="normal">标准</option><option value="large">大</option>
            </select>
          </div>
          <div className="hx-settings-preference-row">
            <div><Activity /><strong>减少动效</strong><small>关闭大部分过渡和进入动画，降低视觉干扰。</small></div>
            <label className="hx-settings-switch"><input type="checkbox" checked={preferences.reduceMotion} onChange={(event) => updatePreferences({ reduceMotion: event.target.checked })} />启用</label>
          </div>
        </div>
        <div className="hx-settings-section-actions"><button type="button" className="hx-btn secondary" onClick={resetPreferences}><RotateCcw />恢复默认</button></div>
      </div>
    </section>

    <section className="hx-settings-section hx-ai-settings">
      <header className="hx-settings-section-head">
        <div><span>密钥与模型</span><h2>AI 服务 · DeepSeek</h2></div>
        <i className={configured ? "configured" : ""}>{configured ? <Check /> : <Bot />}</i>
      </header>
      <form className="hx-settings-section-body hx-settings-form" onSubmit={save}>
        <p className="hx-settings-section-description">管家会按需把相关个人记录发送给 DeepSeek。API Key 只保存在本机数据库，不会返回给页面。</p>
        <label>模型<select
          value={model}
          onChange={(event) => setModel(event.target.value as AISettingsResponse["model"])}
          disabled={loading || saving}
        >
          <option value="deepseek-v4-flash">DeepSeek V4 Flash · 日常推荐</option>
          <option value="deepseek-v4-pro">DeepSeek V4 Pro · 深度分析</option>
        </select></label>
        <label>API Key<div className="hx-secret-input"><KeyRound /><input
          type="password"
          value={apiKey}
          onChange={(event) => setApiKey(event.target.value)}
          autoComplete="new-password"
          placeholder={configured ? "已保存；留空则不修改" : "sk-..."}
          disabled={loading || saving}
        /></div></label>
        <small>默认启用省流检索：先读取摘要、限制记录数，只在必要时获取原文。模型只有读取权限；对话与被读取的记录会发送至 DeepSeek 云端处理。</small>
        <footer>
          {configured && <button type="button" className="hx-btn secondary danger" disabled={saving} onClick={() => void clear()}><Trash2 />移除配置</button>}
          <button className="hx-btn primary" disabled={loading || saving}>{saving ? "保存中…" : "保存 AI 设置"}</button>
        </footer>
        {message && <p className="hx-inline-message" role="status">{message}</p>}
      </form>
    </section>
  </>;
}
