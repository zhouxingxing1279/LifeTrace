"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { X } from "lucide-react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import type { Activity, ActivityType } from "@/src/types";
import {
  CheckinMethodSelector, DateField, MilestoneSettings, ProjectColorPicker, ProjectFormActions,
  ProjectIconPicker, ProjectLivePreview, ProjectScheduleSelector, ProjectTypeSelector,
  RecommendedUnitInput, ReminderSettings,
} from "./ProjectControls";
import {
  createProjectDraft, PROJECT_TYPE_DEFAULTS, projectDraftToActivity, type PersistProjectDraft,
  type ProjectErrors, type ProjectField, validateProjectDraft,
} from "./projectModel";

const focusableSelector = [
  "button:not([disabled])", "input:not([disabled])", "textarea:not([disabled])",
  "select:not([disabled])", "summary", "[tabindex]:not([tabindex='-1'])",
].join(",");

const localDayKey = (date = new Date()) => {
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 10);
};

const streakFromDates = (dates: string[]) => {
  const unique = new Set(dates);
  let streak = 0;
  const cursor = new Date();
  cursor.setHours(12, 0, 0, 0);
  while (unique.has(localDayKey(cursor))) {
    streak += 1;
    cursor.setDate(cursor.getDate() - 1);
  }
  return streak;
};

export default function PersistProjectDialog({ activity, onClose }: { activity?: Activity; onClose: () => void }) {
  const { activities, logs, addActivity, updateActivity } = useLifeStore();
  const mode = activity ? "edit" : "create";
  const initialDraft = useMemo(() => createProjectDraft(activity), [activity]);
  const initialSnapshot = useMemo(() => JSON.stringify(initialDraft), [initialDraft]);
  const [draft, setDraft] = useState<PersistProjectDraft>(initialDraft);
  const [errors, setErrors] = useState<ProjectErrors>({});
  const [touched, setTouched] = useState<Partial<Record<ProjectField, boolean>>>({});
  const [unitTouched, setUnitTouched] = useState(Boolean(activity));
  const [targetTouched, setTargetTouched] = useState(Boolean(activity));
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState("");
  const dialogRef = useRef<HTMLDivElement>(null);
  const nameRef = useRef<HTMLInputElement>(null);
  const restoreFocusRef = useRef<HTMLElement | null>(null);

  const dirty = JSON.stringify(draft) !== initialSnapshot;
  const dirtyRef = useRef(dirty);
  const closeRef = useRef(onClose);
  const allErrors = validateProjectDraft(draft);
  const errorCount = Object.keys(allErrors).length;
  const ownLogs = activity ? logs.filter((log) => log.activityId === activity.id && log.status !== "skipped") : [];
  const todayValue = ownLogs.filter((log) => log.createdAt.startsWith(localDayKey())).reduce((sum, log) => sum + (log.value ?? 1), 0);
  const cumulative = ownLogs.reduce((sum, log) => sum + (log.value ?? 1), 0);
  const streak = streakFromDates(ownLogs.map((log) => log.createdAt.slice(0, 10)));

  const update = (patch: Partial<PersistProjectDraft>) => {
    setDraft((current) => ({ ...current, ...patch }));
    setSubmitError("");
  };

  const requestClose = useCallback(() => {
    if (dirtyRef.current && !window.confirm("当前修改尚未保存，确认退出吗？")) return;
    closeRef.current();
  }, []);

  useEffect(() => {
    dirtyRef.current = dirty;
  }, [dirty]);

  useEffect(() => {
    closeRef.current = onClose;
  }, [onClose]);

  const validateField = (field: ProjectField) => {
    setTouched((current) => ({ ...current, [field]: true }));
    setErrors(validateProjectDraft(draft));
  };

  const setType = (type: ActivityType) => {
    const next: Exclude<ActivityType, "weekly"> = type === "weekly" ? "count" : type;
    const defaults = next === "control" ? { target: 1, unit: "次" } : PROJECT_TYPE_DEFAULTS[next];
    update({
      type: next,
      target: targetTouched ? draft.target : defaults.target,
      unit: unitTouched ? draft.unit : defaults.unit,
    });
  };

  const focusFirstError = (nextErrors: ProjectErrors) => {
    const first = (Object.keys(nextErrors) as ProjectField[])[0];
    if (!first) return;
    const target = dialogRef.current?.querySelector<HTMLElement>(`[data-field="${first}"]`);
    target?.focus();
  };

  useEffect(() => {
    restoreFocusRef.current = document.activeElement as HTMLElement | null;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    nameRef.current?.focus();

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        requestClose();
        return;
      }
      if (event.key !== "Tab" || !dialogRef.current) return;
      const focusable = [...dialogRef.current.querySelectorAll<HTMLElement>(focusableSelector)].filter((element) => !element.hasAttribute("disabled"));
      if (!focusable.length) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };

    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.body.style.overflow = previousOverflow;
      restoreFocusRef.current?.focus();
    };
  }, [requestClose]);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    const nextErrors = validateProjectDraft(draft);
    setTouched(Object.fromEntries(Object.keys(nextErrors).map((key) => [key, true])) as Partial<Record<ProjectField, boolean>>);
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length) {
      focusFirstError(nextErrors);
      return;
    }

    setSubmitting(true);
    setSubmitError("");
    try {
      const data = projectDraftToActivity(draft);
      if (activity) await updateActivity(activity.id, data);
      else await addActivity(data);
      window.dispatchEvent(new CustomEvent("hengxu-toast", { detail: activity ? "坚持项目已更新" : "坚持项目已创建" }));
      onClose();
    } catch (error) {
      setSubmitError(error instanceof Error ? error.message : "项目保存失败，请检查后重试");
      setSubmitting(false);
    }
  };

  return <div className="pp-overlay" onMouseDown={(event) => { if (event.target === event.currentTarget) requestClose(); }}>
    <div ref={dialogRef} className="pp-dialog" role="dialog" aria-modal="true" aria-labelledby="persist-project-title" aria-describedby="persist-project-description">
      <header className="pp-dialog-header">
        <div><span>{mode === "create" ? "建立新的节奏" : "调整坚持节奏"}</span><h2 id="persist-project-title">{mode === "create" ? "创建坚持项目" : "编辑坚持项目"}</h2><p id="persist-project-description">设置目标、建立节奏，让每一次坚持都被看见</p></div>
        <button type="button" aria-label="关闭" onClick={requestClose}><X aria-hidden /></button>
      </header>

      <form className="pp-form" onSubmit={submit} noValidate>
        <div className="pp-dialog-body">
          <aside className="pp-preview-column">
            <ProjectLivePreview draft={draft} activity={activity} todayValue={todayValue} cumulative={cumulative} streak={streak} />
            <ProjectIconPicker value={draft.icon} onChange={(icon) => update({ icon })} />
            <ProjectColorPicker value={draft.color} onChange={(color) => update({ color })} />
          </aside>

          <div className="pp-fields-column">
            <section className="pp-form-section">
              <div className="pp-section-title"><span>01</span><div><h3>项目定义</h3><p>先说清楚你想坚持的是什么</p></div></div>
              <div className="pp-name-field">
                <label htmlFor="project-name">项目名称 <b>*</b></label>
                <input ref={nameRef} id="project-name" data-field="name" required maxLength={30} placeholder="例如：健身训练" value={draft.name} aria-invalid={Boolean((touched.name || errors.name) && allErrors.name)} onBlur={() => validateField("name")} onChange={(event) => update({ name: event.target.value })} />
                <div><span className="pp-error" role="alert">{(touched.name || errors.name) ? allErrors.name : ""}</span><small>{draft.name.length}/30</small></div>
              </div>
              <ProjectTypeSelector value={draft.type} showLegacyControl={activity?.type === "control"} onChange={setType} />
            </section>

            <section className="pp-form-section">
              <div className="pp-section-title"><span>02</span><div><h3>目标与节奏</h3><p>把期待变成可以执行的标准</p></div></div>
              <div className="pp-target-row">
                <div>
                  <label htmlFor="project-target">目标值 <b>*</b></label>
                  <input id="project-target" data-field="target" type="number" min="0.01" step="any" value={draft.target} aria-invalid={Boolean((touched.target || errors.target) && allErrors.target)} onBlur={() => validateField("target")} onChange={(event) => { setTargetTouched(true); update({ target: event.target.value === "" ? "" : Number(event.target.value) }); }} />
                  {(touched.target || errors.target) && <p className="pp-error" role="alert">{allErrors.target}</p>}
                </div>
                <RecommendedUnitInput type={draft.type} value={draft.unit} error={(touched.unit || errors.unit) ? allErrors.unit : undefined} onChange={(unit) => { setUnitTouched(true); update({ unit }); }} />
              </div>
              <div id="project-schedule" data-field="targetDays" tabIndex={-1}><ProjectScheduleSelector draft={draft} update={update} />{(touched.targetDays || errors.targetDays) && <p className="pp-error" role="alert">{allErrors.targetDays}</p>}</div>
              <DateField value={draft.startDate} onChange={(startDate) => update({ startDate })} error={(touched.startDate || errors.startDate) ? allErrors.startDate : undefined} />
            </section>

            <section className="pp-form-section">
              <div className="pp-section-title"><span>03</span><div><h3>记录方式</h3><p>决定完成记录从哪里产生</p></div></div>
              <div id="project-checkin" data-field="syncSource" tabIndex={-1}><CheckinMethodSelector draft={draft} update={update} error={(touched.syncSource || errors.syncSource) ? allErrors.syncSource : undefined} /></div>
              <div className="pp-description-field">
                <label htmlFor="project-description">项目说明</label>
                <textarea id="project-description" maxLength={200} rows={3} placeholder="描述你的目标、动机与执行方式" value={draft.description} onChange={(event) => update({ description: event.target.value })} />
                <small>{draft.description.length}/200</small>
              </div>
            </section>

            <section className="pp-secondary-settings">
              <ReminderSettings />
              <MilestoneSettings />
            </section>
            {activities.some((item) => item.id !== activity?.id && item.checkinMethod === "automatic" && item.syncSource && item.syncSource === draft.syncSource) && draft.checkinMethod === "automatic" &&
              <p className="pp-inline-note">同一数据来源已关联其他项目；一次同步可能同时为多个项目留下记录。</p>}
            {submitError && <div className="pp-submit-error" role="alert">{submitError}</div>}
          </div>
        </div>
        <ProjectFormActions errorCount={errorCount} submitting={submitting} mode={mode} onCancel={requestClose} />
      </form>
    </div>
  </div>;
}
