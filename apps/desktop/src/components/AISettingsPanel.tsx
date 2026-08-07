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

export default function AISettingsPanel({ section = "all" }: { section?: "all" | "preferences" | "ai" }) {
  const [preferences, setPreferences] = useState<AppPreferences>(() => readAppPreferences());
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState<AISettingsResponse["model"]>("deepseek-v4-flash");
  const [configured, setConfigured] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    if (section === "preferences") {
      setLoading(false);
      return;
    }
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
  }, [section]);

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
    {(section === "all" || section === "preferences") && <section className="hx-settings-page-section hx-settings-basic">
      <header><h2>外观</h2><p>调整当前设备上的显示方式，不影响个人数据。</p></header>
      <div className="hx-setting-rows">
        <div className="hx-setting-row"><div><Monitor /><span><strong>主题</strong><small>跟随系统，或固定使用浅色、深色。</small></span></div><select value={preferences.theme} onChange={(event) => updatePreferences({ theme: event.target.value as AppPreferences["theme"] })}><option value="system">跟随系统</option><option value="light">浅色</option><option value="dark">深色</option></select></div>
        <div className="hx-setting-row"><div><Activity /><span><strong>界面密度</strong><small>紧凑模式会减少列表和工具栏留白。</small></span></div><select value={preferences.density} onChange={(event) => updatePreferences({ density: event.target.value as AppPreferences["density"] })}><option value="comfortable">舒适</option><option value="compact">紧凑</option></select></div>
        <div className="hx-setting-row"><div><Type /><span><strong>界面字号</strong><small>统一调整应用基础字号。</small></span></div><select value={preferences.fontScale} onChange={(event) => updatePreferences({ fontScale: event.target.value as AppPreferences["fontScale"] })}><option value="small">小</option><option value="normal">标准</option><option value="large">大</option></select></div>
        <div className="hx-setting-row"><div><Activity /><span><strong>减少动效</strong><small>减少页面过渡和进入动画。</small></span></div><label className="hx-setting-switch"><input type="checkbox" checked={preferences.reduceMotion} onChange={(event) => updatePreferences({ reduceMotion: event.target.checked })} /><span /></label></div>
      </div>
      <footer className="hx-settings-page-actions"><button type="button" className="hx-btn secondary" onClick={resetPreferences}><RotateCcw />恢复默认</button></footer>
    </section>}

    {(section === "all" || section === "ai") && <section className="hx-settings-page-section hx-ai-settings">
      <header><h2>AI 服务</h2><p>配置 LifeTrace AI 管家使用的模型与 API Key。</p></header>
      <form className="hx-settings-standard-form" onSubmit={save}>
        <div className="hx-setting-rows">
          <label className="hx-setting-row"><div><Bot /><span><strong>模型</strong><small>日常建议使用 Flash，需要更深分析时使用 Pro。</small></span></div><select value={model} onChange={(event) => setModel(event.target.value as AISettingsResponse["model"])} disabled={loading || saving}><option value="deepseek-v4-flash">DeepSeek V4 Flash</option><option value="deepseek-v4-pro">DeepSeek V4 Pro</option></select></label>
          <label className="hx-setting-row"><div><KeyRound /><span><strong>API Key</strong><small>{configured ? "已配置；留空保存不会覆盖现有密钥。" : "密钥仅保存在本机数据库。"}</small></span></div><input type="password" value={apiKey} onChange={(event) => setApiKey(event.target.value)} autoComplete="new-password" placeholder={configured ? "已保存" : "sk-..."} disabled={loading || saving} /></label>
        </div>
        {message && <p className="hx-inline-message" role="status">{message}</p>}
        <footer className="hx-settings-page-actions">{configured && <button type="button" className="hx-btn secondary danger" disabled={saving} onClick={() => void clear()}><Trash2 />移除配置</button>}<button className="hx-btn primary" disabled={loading || saving}>{configured ? <Check /> : <Bot />}{saving ? "保存中…" : "保存"}</button></footer>
      </form>
    </section>}
  </>;
}
