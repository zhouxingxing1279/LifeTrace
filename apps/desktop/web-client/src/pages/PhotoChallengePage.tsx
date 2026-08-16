import { useEffect, useMemo, useState } from "react";
import { Camera, CheckCircle2, Cloud, RefreshCw, Trophy } from "lucide-react";
import { API_BASE } from "../cloud/base";
import { browserFetch } from "../cloud/http";
import { Empty, Metric, MetricGrid, Panel } from "../ui";

interface ChallengeStats {
  total: number;
  highScoreCount: number;
  remaining: number;
  target: number;
  achieved: boolean;
  averageScore: number;
}

interface ChallengeEntry {
  id: string;
  fileName?: string | null;
  capturedAt?: string | null;
  score: number;
  qualified: boolean;
  breakdown: Record<string, number>;
  feedback: string;
  model: string;
  thumbnailDataUrl?: string | null;
  scoredAt: string;
  stagingPending: boolean;
}

interface AdminResponse { stats: ChallengeStats; entries: ChallengeEntry[]; }

export function PhotoChallengePage() {
  const [data, setData] = useState<AdminResponse | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);

  async function load() {
    setLoading(true);
    setError("");
    try {
      const response = await browserFetch(`${API_BASE}/api/v1/photo-challenge/admin`, { credentials: "include" });
      const payload = await readJson(response);
      if (!response.ok) throw new Error(errorMessage(payload, `读取摄影挑战失败 (${response.status})`));
      setData(payload as AdminResponse);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "读取摄影挑战失败");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => { void load(); }, []);
  const stats = data?.stats;
  const pending = useMemo(() => data?.entries.filter((entry) => entry.stagingPending).length ?? 0, [data]);
  const best = useMemo(() => data?.entries.reduce((max, entry) => Math.max(max, entry.score), 0) ?? 0, [data]);
  const progress = stats ? Math.min(100, Math.round((stats.highScoreCount / stats.target) * 1000) / 10) : 0;

  return <div className="lt-page-stack pc-admin">
    <section className="pc-admin-hero">
      <div><span className="lt-overline"><Camera /> PHOTO CHALLENGE</span><h2>500+ 高分照片约定</h2><p>只有评分严格超过 90 分的照片计入目标；“超过 500 张”按 501 张达成。</p></div>
      <button className="hx-btn secondary" disabled={loading} onClick={() => void load()}><RefreshCw className={loading ? "spin" : ""} />刷新</button>
    </section>

    {error && <p className="pc-admin-error">{error}</p>}

    <MetricGrid>
      <Metric label="90+ 高分照片" value={`${stats?.highScoreCount ?? 0} / ${stats?.target ?? 501}`} detail={stats?.achieved ? "约定已达成" : `还差 ${stats?.remaining ?? 501} 张`} positive={stats?.achieved} />
      <Metric label="全部提交" value={`${stats?.total ?? 0} 张`} detail={`平均 ${stats?.averageScore?.toFixed(1) ?? "0.0"} 分`} />
      <Metric label="当前最高分" value={`${best} 分`} detail="GLM-4V-Flash 固定评分量表" positive={best > 90} />
      <Metric label="等待电脑接收" value={`${pending} 张`} detail={pending ? "原图仍在云端暂存" : "云端原图已清空"} positive={pending === 0} />
    </MetricGrid>

    <Panel eyebrow="PROGRESS" title="约定进度">
      <div className="pc-admin-progress-copy"><strong>{progress}%</strong><span>{stats?.highScoreCount ?? 0} 张合格照片</span></div>
      <div className="pc-admin-progress"><i style={{ width: `${progress}%` }} /></div>
      <p className="lt-panel-note">电脑端 LifeTrace 会自动拉取暂存原图并写入现有本地相册。默认情况下未被电脑确认接收的原图不会自动过期；只有本地落盘和相册入库成功后，云端原图才会被删除。</p>
    </Panel>

    <Panel eyebrow="RECENT" title="最近评分">
      <div className="pc-admin-grid">
        {data?.entries.map((entry) => <article className={`pc-admin-photo ${entry.qualified ? "qualified" : ""}`} key={entry.id}>
          <div className="pc-admin-thumb">
            {entry.thumbnailDataUrl ? <img src={entry.thumbnailDataUrl} alt={entry.fileName || "评分照片"} /> : <Camera />}
            <b>{entry.score}</b>
          </div>
          <div className="pc-admin-photo-copy">
            <div className="pc-admin-photo-title"><strong>{entry.fileName || "照片"}</strong>{entry.qualified ? <span><Trophy />90+</span> : <span><CheckCircle2 />已评分</span>}</div>
            <p>{entry.feedback}</p>
            <small>{formatTime(entry.scoredAt)} · {entry.model}</small>
            <span className={`pc-cloud-pill ${entry.stagingPending ? "pending" : "saved"}`}><Cloud />{entry.stagingPending ? "等待电脑保存" : "云端原图已清除"}</span>
          </div>
        </article>)}
        {!loading && !data?.entries.length && <Empty title="还没有评分照片" description="她在摄影挑战 PWA 中提交第一张照片后，这里会出现评分和原图转存状态。" />}
      </div>
    </Panel>
  </div>;
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

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("zh-CN", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}
