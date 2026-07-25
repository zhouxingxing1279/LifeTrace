"use client";

import { useState } from "react";
import type { CSSProperties, ComponentType } from "react";
import {
  Bell, BookOpen, Brain, CalendarDays, Check, CheckCircle2, ChevronDown, Clock3,
  Dumbbell, Flag, Footprints, GlassWater, GraduationCap, Hand, Hash, Languages,
  LoaderCircle, Moon, Music2, NotebookPen, RefreshCw, Sparkles, Target,
} from "lucide-react";
import type { Activity, ActivityColorKey, ActivitySyncSource, ActivityType } from "@/src/types";
import type { PersistProjectDraft, ProjectIconId } from "./projectModel";
import { PROJECT_COLOR_KEYS, PROJECT_UNITS, WEEKDAYS } from "./projectModel";

type IconComponent = ComponentType<{ "aria-hidden"?: boolean; className?: string }>;

const ICONS: Record<ProjectIconId, IconComponent> = {
  fitness: Dumbbell,
  running: Footprints,
  reading: BookOpen,
  study: GraduationCap,
  english: Languages,
  piano: Music2,
  meditation: Brain,
  sleep: Moon,
  water: GlassWater,
  target: Target,
  journal: NotebookPen,
  custom: Sparkles,
};

const ICON_LABELS: Record<ProjectIconId, string> = {
  fitness: "健身",
  running: "跑步",
  reading: "阅读",
  study: "学习",
  english: "英语",
  piano: "钢琴",
  meditation: "冥想",
  sleep: "睡眠",
  water: "喝水",
  target: "目标",
  journal: "日记",
  custom: "自定义",
};

export const PROJECT_COLORS: Record<ActivityColorKey, { label: string; value: string; soft: string }> = {
  emerald: { label: "青绿", value: "#137C68", soft: "#E3F3EE" },
  blue: { label: "蓝色", value: "#376FA3", soft: "#E8F0F7" },
  cyan: { label: "青蓝", value: "#2B7D86", soft: "#E3F1F2" },
  violet: { label: "紫色", value: "#746397", soft: "#EFECF5" },
  rose: { label: "粉红", value: "#A65F70", soft: "#F6EAED" },
  orange: { label: "橙色", value: "#B36D36", soft: "#F7EDE4" },
  amber: { label: "黄色", value: "#9B7A27", soft: "#F5F0DF" },
  slate: { label: "灰色", value: "#61706D", soft: "#E9EEEC" },
};

export function ActivityGlyph({ icon, className }: { icon?: string; className?: string }) {
  const Icon = ICONS[icon as ProjectIconId] ?? Target;
  return <Icon aria-hidden className={className} />;
}

export function ProjectLivePreview({
  draft,
  activity,
  todayValue,
  cumulative,
  streak,
}: {
  draft: PersistProjectDraft;
  activity?: Activity;
  todayValue: number;
  cumulative: number;
  streak: number;
}) {
  const color = PROJECT_COLORS[draft.color];
  const target = Number(draft.target) || 1;
  const progress = Math.min(100, Math.max(0, todayValue / target * 100));
  const typeLabel = draft.type === "completion" ? "完成型" : draft.type === "duration" ? "时长型" : draft.type === "count" ? "次数型" : "行为管理";
  const style = { "--project-color": color.value, "--project-soft": color.soft } as CSSProperties;

  return <section className="pp-preview-section" aria-label="项目实时预览">
    <div className="pp-section-heading">
      <div><span>预览效果</span><h3>它会这样出现在坚持列表</h3></div>
      <span className="pp-live"><i />实时</span>
    </div>
    <article className="pp-preview-card" style={style}>
      <header>
        <span className="pp-preview-icon"><ActivityGlyph icon={draft.icon} /></span>
        <span className="pp-preview-type">{typeLabel}</span>
      </header>
      <h2>{draft.name.trim() || "健身训练"}</h2>
      <p>{draft.description.trim() || "完成一次训练，即记录今天的坚持"}</p>
      <div className="pp-preview-progress">
        <span>今日进度</span>
        <strong>{todayValue} / {target} {draft.unit || "次"}</strong>
      </div>
      <div className="pp-preview-track" aria-label={`今日完成 ${Math.round(progress)}%`}><i style={{ width: `${progress}%` }} /></div>
      <footer>
        <span><b>{cumulative}</b>累计完成</span>
        <span><b>{streak}</b>连续天数</span>
        <small>{activity ? "已保存项目" : "从今天开始"}</small>
      </footer>
    </article>
  </section>;
}

export function ProjectIconPicker({ value, onChange }: { value: ProjectIconId; onChange: (value: ProjectIconId) => void }) {
  const [showMore, setShowMore] = useState(false);
  const iconIds = Object.keys(ICONS) as ProjectIconId[];
  const visibleIcons = showMore ? iconIds : iconIds.slice(0, 8);

  return <section className="pp-personalize" aria-labelledby="project-icon-label">
    <div className="pp-control-label" id="project-icon-label"><span>项目图标</span><small>选择一个一眼能认出的符号</small></div>
    <div className="pp-icon-grid">
      {visibleIcons.map((id) => {
        const Icon = ICONS[id];
        return <button type="button" key={id} className={value === id ? "selected" : ""} aria-pressed={value === id} onClick={() => onChange(id)}>
          <Icon aria-hidden />
          <span>{ICON_LABELS[id]}</span>
        </button>;
      })}
    </div>
    <button className="pp-more-icons" type="button" aria-expanded={showMore} onClick={() => setShowMore((current) => !current)}>
      {showMore ? "收起图标" : "更多图标"}
      <ChevronDown aria-hidden className={showMore ? "expanded" : ""} />
    </button>
  </section>;
}

export function ProjectColorPicker({ value, onChange }: { value: ActivityColorKey; onChange: (value: ActivityColorKey) => void }) {
  return <section className="pp-personalize" aria-labelledby="project-color-label">
    <div className="pp-control-label" id="project-color-label"><span>主题颜色</span><small>{PROJECT_COLORS[value].label}</small></div>
    <div className="pp-color-row">
      {PROJECT_COLOR_KEYS.map((key) => {
        const color = PROJECT_COLORS[key];
        return <button type="button" key={key} className={value === key ? "selected" : ""} aria-label={color.label} aria-pressed={value === key} style={{ "--swatch": color.value } as CSSProperties} onClick={() => onChange(key)}>
          {value === key && <Check aria-hidden />}
        </button>;
      })}
    </div>
  </section>;
}

const TYPE_OPTIONS: { value: "completion" | "duration" | "count"; title: string; description: string; example: string; icon: IconComponent }[] = [
  { value: "completion", title: "完成型", description: "完成一次即视为当天达成", example: "健身、早起、阅读打卡", icon: CheckCircle2 },
  { value: "duration", title: "时长型", description: "记录当天累计持续时间", example: "学习、练琴、冥想", icon: Clock3 },
  { value: "count", title: "次数型", description: "累计当天完成次数", example: "喝水、背单词、俯卧撑", icon: Hash },
];

export function ProjectTypeSelector({ value, showLegacyControl, onChange }: { value: ActivityType; showLegacyControl: boolean; onChange: (value: ActivityType) => void }) {
  return <fieldset className="pp-fieldset">
    <legend>项目类型 <b>*</b></legend>
    <div className="pp-type-grid">
      {TYPE_OPTIONS.map(({ value: id, title, description, example, icon: Icon }) =>
        <button type="button" key={id} className={value === id ? "selected" : ""} aria-pressed={value === id} onClick={() => onChange(id)}>
          <span className="pp-type-icon"><Icon aria-hidden /></span>
          <span><strong>{title}</strong><small>{description}</small><em>适合：{example}</em></span>
          {value === id && <Check className="pp-selected-check" aria-hidden />}
        </button>)}
      {showLegacyControl && <button type="button" className={value === "control" ? "selected legacy" : "legacy"} aria-pressed={value === "control"} onClick={() => onChange("control")}>
        <span className="pp-type-icon"><Hand aria-hidden /></span>
        <span><strong>行为管理</strong><small>保留原项目的状态记录方式</small><em>兼容已有数据</em></span>
        {value === "control" && <Check className="pp-selected-check" aria-hidden />}
      </button>}
    </div>
  </fieldset>;
}

export function ProjectScheduleSelector({ draft, update }: { draft: PersistProjectDraft; update: (patch: Partial<PersistProjectDraft>) => void }) {
  const setSchedule = (scheduleType: PersistProjectDraft["scheduleType"]) => {
    update({
      scheduleType,
      targetDays: scheduleType === "daily"
        ? WEEKDAYS.map((day) => day.value)
        : draft.targetDays.length && draft.targetDays.length < 7 ? draft.targetDays : [1, 3, 5],
    });
  };
  const toggleDay = (day: number) => update({ targetDays: draft.targetDays.includes(day) ? draft.targetDays.filter((value) => value !== day) : [...draft.targetDays, day] });

  return <fieldset className="pp-fieldset pp-schedule">
    <legend>执行周期 <b>*</b></legend>
    <div className="pp-segmented" role="group" aria-label="执行周期">
      {([["daily", "每日"], ["weekly", "每周"], ["custom", "自定义"]] as const).map(([id, label]) =>
        <button type="button" key={id} className={draft.scheduleType === id ? "selected" : ""} aria-pressed={draft.scheduleType === id} onClick={() => setSchedule(id)}>{label}</button>)}
    </div>
    {draft.scheduleType !== "daily" && <div className="pp-weekdays" aria-label="选择执行星期">
      {WEEKDAYS.map((day) => <button type="button" key={day.value} title={day.label} className={draft.targetDays.includes(day.value) ? "selected" : ""} aria-pressed={draft.targetDays.includes(day.value)} onClick={() => toggleDay(day.value)}>{day.short}</button>)}
    </div>}
  </fieldset>;
}

const SYNC_SOURCES: { value: ActivitySyncSource; label: string; description: string }[] = [
  { value: "fitness", label: "健身训练", description: "训记导入或训练完成后记录" },
  { value: "english", label: "每日英语", description: "完成阅读、总结与 AI 反馈后记录" },
];

export function CheckinMethodSelector({ draft, update, error }: { draft: PersistProjectDraft; update: (patch: Partial<PersistProjectDraft>) => void; error?: string }) {
  return <fieldset className="pp-fieldset">
    <legend>打卡方式 <b>*</b></legend>
    <div className="pp-checkin-grid">
      <button type="button" className={draft.checkinMethod === "manual" ? "selected" : ""} aria-pressed={draft.checkinMethod === "manual"} onClick={() => update({ checkinMethod: "manual", syncSource: "" })}>
        <Hand aria-hidden /><span><strong>手动记录</strong><small>主动点击完成或记录数据</small></span>
      </button>
      <button type="button" className={draft.checkinMethod === "automatic" ? "selected" : ""} aria-pressed={draft.checkinMethod === "automatic"} onClick={() => update({ checkinMethod: "automatic", syncSource: draft.syncSource || "fitness" })}>
        <RefreshCw aria-hidden /><span><strong>自动同步</strong><small>由现有业务模块生成记录</small></span>
      </button>
    </div>
    {draft.checkinMethod === "automatic" && <div className="pp-sync-sources">
      <span>数据来源</span>
      {SYNC_SOURCES.map((source) => <label key={source.value} className={draft.syncSource === source.value ? "selected" : ""}>
        <input type="radio" name="sync-source" value={source.value} checked={draft.syncSource === source.value} onChange={() => update({ syncSource: source.value })} />
        <span><b>{source.label}</b><small>{source.description}</small></span>
      </label>)}
    </div>}
    {error && <p className="pp-error" role="alert">{error}</p>}
  </fieldset>;
}

export function RecommendedUnitInput({ type, value, onChange, error }: { type: ActivityType; value: string; onChange: (value: string) => void; error?: string }) {
  const units = type === "control" || type === "weekly" ? ["次"] : PROJECT_UNITS[type];
  return <div className="pp-unit-field">
    <label htmlFor="project-unit">单位 <b>*</b></label>
    <input id="project-unit" data-field="unit" list="project-unit-options" maxLength={20} value={value} onChange={(event) => onChange(event.target.value)} aria-invalid={Boolean(error)} />
    <datalist id="project-unit-options">{units.map((unit) => <option value={unit} key={unit} />)}</datalist>
    <div className="pp-unit-suggestions">{units.map((unit) => <button type="button" key={unit} className={value === unit ? "selected" : ""} onClick={() => onChange(unit)}>{unit}</button>)}</div>
    {error && <p className="pp-error" role="alert">{error}</p>}
  </div>;
}

export function ReminderSettings() {
  return <details className="pp-disclosure">
    <summary><span><Bell aria-hidden /><span><strong>提醒设置</strong><small>在合适的时间提醒你完成今天的目标</small></span></span><span className="pp-summary-status">暂未启用</span><ChevronDown aria-hidden /></summary>
    <div className="pp-disabled-setting">
      <p>当前桌面端尚未接入系统通知服务。此处保留设置位置，但不会创建假的提醒。</p>
      <label><input type="checkbox" disabled /> 开启项目提醒</label>
      <div><label>提醒时间<input type="time" disabled value="20:00" readOnly /></label><label>提醒文案<input disabled value="记得完成今天的坚持" readOnly /></label></div>
    </div>
  </details>;
}

export function MilestoneSettings() {
  return <details className="pp-disclosure">
    <summary><span><Flag aria-hidden /><span><strong>里程碑设置</strong><small>为长期坚持设置值得庆祝的节点</small></span></span><span className="pp-summary-status">暂未启用</span><ChevronDown aria-hidden /></summary>
    <div className="pp-disabled-setting">
      <p>当前数据库尚未建立里程碑实体，因此不会把这些选项写入项目主记录。</p>
      {["连续坚持 7 天", "累计完成 30 次", "连续坚持 30 天"].map((label) => <label key={label}><input type="checkbox" disabled /> {label}</label>)}
    </div>
  </details>;
}

export function ProjectFormActions({ errorCount, submitting, mode, onCancel }: { errorCount: number; submitting: boolean; mode: "create" | "edit"; onCancel: () => void }) {
  return <footer className="pp-actions">
    <div className={errorCount ? "invalid" : "valid"} aria-live="polite">
      {errorCount ? <><span>{errorCount}</span> 项必填内容需要完善</> : <><CheckCircle2 aria-hidden />所有设置已完成</>}
    </div>
    <div>
      <button type="button" className="hx-btn secondary" onClick={onCancel} disabled={submitting}>取消</button>
      <button type="submit" className="hx-btn primary" disabled={submitting}>
        {submitting && <LoaderCircle className="pp-spin" aria-hidden />}
        {submitting ? "正在保存…" : mode === "create" ? "创建项目" : "保存修改"}
      </button>
    </div>
  </footer>;
}

export function DateField({ value, onChange, error }: { value: string; onChange: (value: string) => void; error?: string }) {
  return <div className="pp-date-field">
    <label htmlFor="project-start-date"><CalendarDays aria-hidden />开始日期 <b>*</b></label>
    <input id="project-start-date" data-field="startDate" type="date" value={value} onChange={(event) => onChange(event.target.value)} aria-invalid={Boolean(error)} />
    <small>暂无结束日期，按你的节奏长期积累。</small>
    {error && <p className="pp-error" role="alert">{error}</p>}
  </div>;
}
