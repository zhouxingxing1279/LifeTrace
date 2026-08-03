"use client";

import { useEffect, useState } from "react";
import { Bot, Check, KeyRound, Trash2 } from "lucide-react";

type AISettingsResponse = {
  provider: "deepseek";
  model: "deepseek-v4-flash" | "deepseek-v4-pro";
  configured: boolean;
  updatedAt?: string;
  error?: string;
};

export default function AISettingsPanel() {
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

  return <article className="hx-panel hx-ai-settings">
    <header className="hx-panel-head">
      <div><span>AI 管家</span><h2>DeepSeek</h2></div>
      <i className={configured ? "configured" : ""}>{configured ? <Check /> : <Bot />}</i>
    </header>
    <form className="hx-panel-body" onSubmit={save}>
      <p>管家会按需把相关个人记录发送给 DeepSeek。API Key 只保存在本机数据库，不会返回给页面。</p>
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
  </article>;
}
