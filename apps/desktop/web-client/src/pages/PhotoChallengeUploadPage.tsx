import { useEffect, useMemo, useRef, useState } from "react";
import { Camera, CheckCircle2, ImagePlus, KeyRound, LoaderCircle, Sparkles, Trophy } from "lucide-react";
import { API_BASE } from "../cloud/base";

interface ChallengeStats {
  total: number;
  highScoreCount: number;
  remaining: number;
  target: number;
  achieved: boolean;
  averageScore: number;
}

interface ScoreBreakdown {
  composition: number;
  lightColor: number;
  subjectStory: number;
  technical: number;
  originality: number;
}

interface ScoreResult {
  id: string;
  score: number;
  qualified: boolean;
  breakdown: ScoreBreakdown;
  feedback: string;
  duplicate: boolean;
  stats: ChallengeStats;
}

const KEY_STORAGE = "lifetrace.photoChallenge.accessKey";

export function PhotoChallengeUploadPage() {
  const inputRef = useRef<HTMLInputElement>(null);
  const [accessKey, setAccessKey] = useState(() => localStorage.getItem(KEY_STORAGE) ?? "");
  const [stats, setStats] = useState<ChallengeStats | null>(null);
  const [result, setResult] = useState<ScoreResult | null>(null);
  const [preview, setPreview] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const progress = useMemo(() => stats ? Math.min(100, (stats.highScoreCount / stats.target) * 100) : 0, [stats]);

  useEffect(() => {
    const manifest = document.createElement("link");
    manifest.rel = "manifest";
    manifest.href = "/photo-challenge.webmanifest";
    document.head.appendChild(manifest);
    document.title = "摄影挑战 · LifeTrace";
    if ("serviceWorker" in navigator) navigator.serviceWorker.register("/photo-challenge-sw.js").catch(() => undefined);
    return () => manifest.remove();
  }, []);

  useEffect(() => {
    if (!accessKey.trim()) return;
    void loadStats(accessKey.trim()).then(setStats).catch(() => undefined);
  }, []);

  async function saveKey() {
    const value = accessKey.trim();
    if (!value) { setMessage("请输入挑战口令"); return; }
    setBusy(true);
    setMessage("");
    try {
      const next = await loadStats(value);
      localStorage.setItem(KEY_STORAGE, value);
      setStats(next);
    } catch (cause) {
      setMessage(cause instanceof Error ? cause.message : "挑战口令无效");
    } finally {
      setBusy(false);
    }
  }

  async function choosePhoto(next: File | null) {
    setResult(null);
    setMessage("");
    if (!next) return;
    if (!/^image\/(jpeg|png|webp)$/.test(next.type)) {
      setMessage("请选择 JPEG、PNG 或 WebP 照片");
      return;
    }
    if (next.size > 64 * 1024 * 1024) {
      setMessage("原图不能超过 64 MiB");
      return;
    }
    setFile(next);
    setPreview(URL.createObjectURL(next));
  }

  async function submit() {
    const key = accessKey.trim();
    if (!key) { setMessage("请先输入挑战口令"); return; }
    if (!file) { setMessage("请先拍摄或选择一张照片"); return; }
    setBusy(true);
    setMessage("正在准备照片并交给 GLM-4V-Flash 评分…");
    try {
      const [modelPreview, thumbnail] = await Promise.all([
        resizeImage(file, 1600, 0.86),
        resizeImage(file, 360, 0.68),
      ]);
      const form = new FormData();
      form.append("file", file, file.name || "photo.jpg");
      form.append("previewDataUrl", modelPreview);
      form.append("thumbnailDataUrl", thumbnail);
      if (file.lastModified) form.append("capturedAt", new Date(file.lastModified).toISOString());
      const response = await fetch(`${API_BASE}/api/v1/photo-challenge/score`, {
        method: "POST",
        headers: { "x-photo-challenge-key": key },
        body: form,
      });
      const payload = await readJson(response);
      if (!response.ok) throw new Error(errorMessage(payload, `评分失败 (${response.status})`));
      const scored = payload as ScoreResult;
      localStorage.setItem(KEY_STORAGE, key);
      setResult(scored);
      setStats(scored.stats);
      setMessage(scored.duplicate ? "这张照片以前已经提交过，本次不会重复计数。" : "原图已进入 LifeTrace 云端中转队列。电脑保存成功后，云端原图会自动删除。\n");
    } catch (cause) {
      setMessage(cause instanceof Error ? cause.message : "照片评分失败，请稍后重试");
    } finally {
      setBusy(false);
    }
  }

  return <main className="pc-public-shell">
    <section className="pc-public-card">
      <header className="pc-public-header">
        <span className="pc-public-mark"><Camera /></span>
        <div><small>LIFETRACE · PHOTO CHALLENGE</small><h1>拍到 501 张真正优秀的照片</h1></div>
      </header>

      <div className="pc-progress-card">
        <div><span>90+ 高分照片</span><strong>{stats?.highScoreCount ?? "—"} <small>/ {stats?.target ?? 501}</small></strong></div>
        <div className="pc-progress-track"><i style={{ width: `${progress}%` }} /></div>
        <p>{stats ? (stats.achieved ? "约定已达成 🎉" : `还差 ${stats.remaining} 张超过 90 分的照片`) : "输入挑战口令后显示实时进度"}</p>
      </div>

      {!stats && <section className="pc-key-panel">
        <KeyRound />
        <div><strong>挑战口令</strong><p>口令只保存在这台手机浏览器里，不是 LifeTrace 账号密码。</p></div>
        <input value={accessKey} onChange={(event) => setAccessKey(event.target.value)} type="password" placeholder="输入口令" autoComplete="off" />
        <button disabled={busy} onClick={() => void saveKey()}>进入挑战</button>
      </section>}

      {stats && <>
        <button className="pc-photo-picker" onClick={() => inputRef.current?.click()} disabled={busy}>
          {preview ? <img src={preview} alt="待评分照片预览" /> : <span><ImagePlus /><b>拍摄或选择照片</b><small>原图会暂存在云端并自动同步到 LifeTrace 相册</small></span>}
        </button>
        <input ref={inputRef} hidden type="file" accept="image/jpeg,image/png,image/webp" onChange={(event) => void choosePhoto(event.target.files?.[0] ?? null)} />

        {file && <div className="pc-file-row"><div><strong>{file.name || "照片"}</strong><small>{formatBytes(file.size)}</small></div><button onClick={() => inputRef.current?.click()} disabled={busy}>换一张</button></div>}

        <button className="pc-score-button" disabled={!file || busy} onClick={() => void submit()}>
          {busy ? <LoaderCircle className="spin" /> : <Sparkles />}{busy ? "正在评分" : "提交并评分"}
        </button>
      </>}

      {result && <section className={`pc-result ${result.qualified ? "qualified" : ""}`}>
        <div className="pc-score-orb"><strong>{result.score}</strong><span>/ 100</span></div>
        <div className="pc-result-copy">
          <h2>{result.qualified ? <><Trophy />超过 90 分，计入约定</> : <><CheckCircle2 />这次还没超过 90 分</>}</h2>
          <p>{result.feedback}</p>
          <div className="pc-breakdown">
            <ScorePart label="构图" value={result.breakdown.composition} max={25} />
            <ScorePart label="光线色彩" value={result.breakdown.lightColor} max={20} />
            <ScorePart label="主体叙事" value={result.breakdown.subjectStory} max={20} />
            <ScorePart label="技术质量" value={result.breakdown.technical} max={20} />
            <ScorePart label="原创瞬间" value={result.breakdown.originality} max={15} />
          </div>
        </div>
      </section>}

      {message && <p className="pc-message">{message}</p>}
      <footer>超过 90 分才计数 · 同一张照片不会重复计数 · 目标是超过 500 张，因此达成数为 501 张</footer>
    </section>
  </main>;
}

function ScorePart({ label, value, max }: { label: string; value: number; max: number }) {
  return <div><span>{label}</span><b>{value}<small>/{max}</small></b></div>;
}

async function loadStats(key: string): Promise<ChallengeStats> {
  const response = await fetch(`${API_BASE}/api/v1/photo-challenge/summary`, { headers: { "x-photo-challenge-key": key } });
  const payload = await readJson(response);
  if (!response.ok) throw new Error(errorMessage(payload, `无法读取挑战进度 (${response.status})`));
  return payload as ChallengeStats;
}

async function resizeImage(file: File, maxEdge: number, quality: number): Promise<string> {
  const bitmap = await createImageBitmap(file);
  const scale = Math.min(1, maxEdge / Math.max(bitmap.width, bitmap.height));
  const width = Math.max(1, Math.round(bitmap.width * scale));
  const height = Math.max(1, Math.round(bitmap.height * scale));
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("浏览器无法处理这张照片");
  context.fillStyle = "#ffffff";
  context.fillRect(0, 0, width, height);
  context.drawImage(bitmap, 0, 0, width, height);
  bitmap.close();
  return canvas.toDataURL("image/jpeg", quality);
}

async function readJson(response: Response): Promise<unknown> {
  const text = await response.text();
  if (!text) return null;
  try { return JSON.parse(text) as unknown; } catch { return { message: text }; }
}

function errorMessage(payload: unknown, fallback: string): string {
  if (!payload || typeof payload !== "object") return fallback;
  const value = payload as Record<string, unknown>;
  if (typeof value.message === "string") return value.message;
  const error = value.error;
  if (error && typeof error === "object" && typeof (error as Record<string, unknown>).message === "string") return String((error as Record<string, unknown>).message);
  return fallback;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
