"use client";

import { useRef, useState } from "react";
import { Check, ChevronDown, FileImage, Pencil, Plus, QrCode, RotateCcw, Trash2, X } from "lucide-react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import type { XunjiWorkout } from "@/src/types";

type ParsedImport = {
  importId: string;
  shareUrl: string;
  parser: "embedded-json" | "dom";
  workout: XunjiWorkout;
};

const cloneWorkout = (workout: XunjiWorkout) => JSON.parse(JSON.stringify(workout)) as XunjiWorkout;

// 用户确认前只保留预览草稿；正式记录由服务端事务统一写入 SQLite。
export default function XunjiImportPanel() {
  const initialize = useLifeStore((state) => state.initialize);
  const fileInput = useRef<HTMLInputElement>(null);
  const [expanded, setExpanded] = useState(false);
  const [loading, setLoading] = useState(false);
  const [parsed, setParsed] = useState<ParsedImport | null>(null);
  const [draft, setDraft] = useState<XunjiWorkout | null>(null);
  const [editing, setEditing] = useState(false);
  const [message, setMessage] = useState("");
  const [success, setSuccess] = useState(false);

  const upload = async (file?: File) => {
    if (!file) return;
    setLoading(true);
    setMessage("");
    setSuccess(false);
    try {
      const form = new FormData();
      form.set("image", file);
      const response = await fetch("/api/xunji/parse", { method: "POST", body: form });
      const payload = await response.json() as ParsedImport & { error?: string };
      if (!response.ok) throw new Error(payload.error ?? "训记分享解析失败");
      setParsed(payload);
      setDraft(cloneWorkout(payload.workout));
      setEditing(false);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "训记分享解析失败");
    } finally {
      setLoading(false);
      if (fileInput.current) fileInput.current.value = "";
    }
  };

  const finish = async (action: "confirm" | "cancel") => {
    if (!parsed || !draft) return;
    setLoading(true);
    setMessage("");
    try {
      const response = await fetch("/api/xunji/imports", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ importId: parsed.importId, action, workout: action === "confirm" ? draft : undefined }),
      });
      const payload = await response.json() as { error?: string };
      if (!response.ok) throw new Error(payload.error ?? "训练导入失败");
      if (action === "confirm") {
        await initialize();
        setSuccess(true);
      }
      setParsed(null);
      setDraft(null);
      setEditing(false);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "训练导入失败");
    } finally {
      setLoading(false);
    }
  };

  const updateWorkout = (patch: Partial<XunjiWorkout>) => setDraft((value) => value ? { ...value, ...patch } : value);
  const updateExercise = (exerciseIndex: number, name: string) => setDraft((value) => value ? {
    ...value,
    exercises: value.exercises.map((exercise, index) => index === exerciseIndex ? { ...exercise, name } : exercise),
  } : value);
  const updateSet = (exerciseIndex: number, setIndex: number, patch: { weightKg?: number; reps?: number }) => setDraft((value) => value ? {
    ...value,
    exercises: value.exercises.map((exercise, index) => index === exerciseIndex ? {
      ...exercise,
      sets: exercise.sets.map((set, indexOfSet) => indexOfSet === setIndex ? { ...set, ...patch } : set),
    } : exercise),
  } : value);
  return <section className={`xj-panel ${expanded ? "expanded" : ""}`}>
    <button className="xj-panel-head" onClick={() => setExpanded((value) => !value)}>
      <span><i><QrCode /></i><span><b>训记训练数据导入</b><small>选择分享图 · 扫描二维码 · 确认后导入</small></span></span>
      <ChevronDown />
    </button>
    {expanded && <div className="xj-panel-body">
      {!parsed && <div className="xj-start">
        <div><FileImage /><h3>{success ? "训练已导入并完成联动" : "上传训记分享图片"}</h3><p>图片仅用于读取二维码，不会识别图片文字，也不会保存原图。</p></div>
        <button className="hx-btn primary" disabled={loading} onClick={() => fileInput.current?.click()}>{loading ? "正在解析…" : success ? "继续导入" : "选择图片"}</button>
        <input ref={fileInput} hidden type="file" accept="image/jpeg,image/png,image/webp,image/bmp" onChange={(event) => void upload(event.target.files?.[0])} />
      </div>}
      {message && <div className="xj-error"><X /><span><b>无法完成解析</b>{message}</span><button onClick={() => setMessage("")}>关闭</button></div>}
      {parsed && draft && <div className="xj-preview">
        <header><div><span className="xj-success"><Check />解析成功</span><h3>{editing ? "编辑训练数据" : draft.title}</h3><small>解析方式：{parsed.parser === "embedded-json" ? "网页内嵌数据" : "网页结构"}</small></div><button className="hx-btn secondary" onClick={() => setEditing((value) => !value)}><Pencil />{editing ? "完成编辑" : "编辑"}</button></header>
        <div className="xj-meta">
          <label>训练日期<input disabled={!editing} type="date" value={draft.date} onChange={(event) => updateWorkout({ date: event.target.value })} /></label>
          <label>训练名称<input disabled={!editing} value={draft.title} onChange={(event) => updateWorkout({ title: event.target.value })} /></label>
          <label>时长（分钟）<input disabled={!editing} type="number" min="0" value={draft.durationMinutes} onChange={(event) => updateWorkout({ durationMinutes: Number(event.target.value) })} /></label>
          <label>热量（千卡）<input disabled={!editing} type="number" min="0" value={draft.caloriesKcal} onChange={(event) => updateWorkout({ caloriesKcal: Number(event.target.value) })} /></label>
          <label>总容量（千克）<input disabled={!editing} type="number" min="0" value={draft.volumeKg} onChange={(event) => updateWorkout({ volumeKg: Number(event.target.value) })} /></label>
        </div>
        <div className="xj-exercises">{draft.exercises.map((exercise, exerciseIndex) => <article key={`${exercise.name}-${exerciseIndex}`}>
          <header><input disabled={!editing} value={exercise.name} onChange={(event) => updateExercise(exerciseIndex, event.target.value)} /><b>{exercise.sets.length} 组</b>{editing && <button aria-label={`删除${exercise.name}`} onClick={() => setDraft((value) => value ? { ...value, exercises: value.exercises.filter((_, index) => index !== exerciseIndex) } : value)}><Trash2 /></button>}</header>
          <div><span>组数</span><span>重量（千克）</span><span>次数</span>{exercise.sets.map((set, setIndex) => <div key={set.setNumber}><b>第 {setIndex + 1} 组</b><input disabled={!editing} type="number" min="0" step=".5" value={set.weightKg} onChange={(event) => updateSet(exerciseIndex, setIndex, { weightKg: Number(event.target.value) })} /><input disabled={!editing} type="number" min="0" value={set.reps} onChange={(event) => updateSet(exerciseIndex, setIndex, { reps: Number(event.target.value) })} /></div>)}</div>
          {editing && <button className="xj-add-set" onClick={() => setDraft((value) => value ? { ...value, exercises: value.exercises.map((item, index) => index === exerciseIndex ? { ...item, sets: [...item.sets, { weightKg: 0, reps: 10, setNumber: item.sets.length + 1 }] } : item) } : value)}><Plus />增加一组</button>}
        </article>)}</div>
        <footer><button className="hx-btn secondary" disabled={loading} onClick={() => void finish("cancel")}>取消</button><button className="hx-btn secondary" onClick={() => { setDraft(cloneWorkout(parsed.workout)); setEditing(false); }}><RotateCcw />恢复解析结果</button><button className="hx-btn primary" disabled={loading || !draft.title.trim() || !draft.exercises.length} onClick={() => void finish("confirm")}><Check />{loading ? "正在导入…" : "确认导入"}</button></footer>
      </div>}
    </div>}
  </section>;
}
