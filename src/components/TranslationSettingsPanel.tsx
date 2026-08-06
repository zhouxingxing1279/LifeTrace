"use client";

import { useEffect, useState } from "react";
import { Check, KeyRound, Languages, Trash2 } from "lucide-react";

type TranslationSettingsResponse = {
  appId: string;
  configured: boolean;
  updatedAt?: string;
  error?: string;
};

export default function TranslationSettingsPanel() {
  const [appId, setAppId] = useState("");
  const [secret, setSecret] = useState("");
  const [configured, setConfigured] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    const controller = new AbortController();
    void fetch("/api/settings/translation", { signal: controller.signal })
      .then(async (response) => {
        const payload = await response.json() as TranslationSettingsResponse;
        if (!response.ok) throw new Error(payload.error || "无法读取翻译设置");
        setAppId(payload.appId);
        setConfigured(payload.configured);
      })
      .catch((error) => {
        if (!controller.signal.aborted) setMessage(error instanceof Error ? error.message : "无法读取翻译设置");
      })
      .finally(() => setLoading(false));
    return () => controller.abort();
  }, []);

  const save = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!appId.trim() || (!secret.trim() && !configured)) {
      setMessage("请填写百度翻译 APPID 和密钥");
      return;
    }
    setSaving(true);
    setMessage("");
    try {
      const response = await fetch("/api/settings/translation", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ appId, secret: secret || undefined }),
      });
      const payload = await response.json() as TranslationSettingsResponse;
      if (!response.ok) throw new Error(payload.error || "翻译设置保存失败");
      setAppId(payload.appId);
      setConfigured(payload.configured);
      setSecret("");
      setMessage("百度翻译配置已保存到本机");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "翻译设置保存失败");
    } finally {
      setSaving(false);
    }
  };

  const clear = async () => {
    if (!window.confirm("移除本机保存的百度翻译配置？")) return;
    setSaving(true);
    setMessage("");
    try {
      const response = await fetch("/api/settings/translation", { method: "DELETE" });
      if (!response.ok) throw new Error("翻译设置清除失败");
      setAppId("");
      setSecret("");
      setConfigured(false);
      setMessage("百度翻译配置已移除");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "翻译设置清除失败");
    } finally {
      setSaving(false);
    }
  };

  return <section className="hx-settings-section hx-translation-settings">
    <header className="hx-settings-section-head">
      <div><span>密钥与服务</span><h2>翻译服务 · 百度翻译</h2></div>
      <i className={configured ? "configured" : ""}>{configured ? <Check /> : <Languages />}</i>
    </header>
    <form className="hx-settings-section-body hx-settings-form" onSubmit={save}>
      <p className="hx-settings-section-description">用于每日英语的划句翻译。APPID 和密钥统一在设置中管理，密钥只保存在本机数据库，阅读页面无法读取。</p>
      <label>APPID<input
        value={appId}
        onChange={(event) => setAppId(event.target.value)}
        autoComplete="off"
        placeholder="百度翻译开放平台 APPID"
        disabled={loading || saving}
      /></label>
      <label>密钥<div className="hx-secret-input"><KeyRound /><input
        type="password"
        value={secret}
        onChange={(event) => setSecret(event.target.value)}
        autoComplete="new-password"
        placeholder={configured ? "已保存；留空则不修改" : "百度翻译开放平台密钥"}
        disabled={loading || saving}
      /></div></label>
      <footer>
        {configured && <button type="button" className="hx-btn secondary danger" disabled={saving} onClick={() => void clear()}><Trash2 />移除配置</button>}
        <button className="hx-btn primary" disabled={loading || saving}>{saving ? "保存中…" : "保存翻译设置"}</button>
      </footer>
      {message && <p className="hx-inline-message" role="status">{message}</p>}
    </form>
  </section>;
}
