import { useMemo, useRef, useState } from "react";
import {
  Check,
  Copy,
  Download,
  Edit3,
  Import,
  Palette,
  Plus,
  RotateCcw,
  Save,
  Trash2,
} from "lucide-react";
import { PanelHead } from "@/src/components/common";
import {
  applySkin,
  cloneSkin,
  contrastRatio,
  deleteCustomSkin,
  exportSkinDocument,
  getSkinLibrary,
  importSkinDocument,
  readActiveSkinId,
  resolveSkin,
  saveCustomSkin,
  type SkinDefinition,
  type SkinPalette,
  writeActiveSkinId,
} from "@/src/services/skinEngine";
import {
  applyAppPreferences,
  readAppPreferences,
  writeAppPreferences,
  type AppPreferences,
  type ThemePreference,
} from "@/src/services/appPreferences";

type PaletteKey = keyof SkinPalette;

type ColorField = {
  key: PaletteKey;
  label: string;
};

const COLOR_FIELDS: readonly ColorField[] = [
  { key: "background", label: "应用背景" },
  { key: "surface", label: "面板" },
  { key: "foreground", label: "正文" },
  { key: "muted", label: "次级文字" },
  { key: "border", label: "边框" },
  { key: "primary", label: "强调色" },
  { key: "success", label: "成功" },
  { key: "warning", label: "警告" },
  { key: "danger", label: "危险" },
  { key: "info", label: "信息" },
];

function notify(message: string, type: "success" | "error" = "success") {
  window.dispatchEvent(
    new CustomEvent("hengxu-toast", {
      detail: { message, type, duration: type === "error" ? 4500 : 2500 },
    }),
  );
}

function downloadText(text: string, filename: string) {
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

function SkinPreview({ skin }: { skin: SkinDefinition }) {
  return (
    <div
      className="lt-skin-preview"
      style={{
        "--preview-bg": skin.light.background,
        "--preview-surface": skin.light.surface,
        "--preview-fg": skin.light.foreground,
        "--preview-muted": skin.light.muted,
        "--preview-border": skin.light.border,
        "--preview-primary": skin.light.primary,
      } as React.CSSProperties}
      aria-hidden="true"
    >
      <span className="lt-skin-preview-sidebar">
        <i />
        <i />
        <i />
      </span>
      <span className="lt-skin-preview-content">
        <b />
        <i />
        <i />
      </span>
    </div>
  );
}

function ThemeModeSelector({
  value,
  onChange,
}: {
  value: ThemePreference;
  onChange: (value: ThemePreference) => void;
}) {
  return (
    <div className="lt-appearance-segment" role="group" aria-label="显示模式">
      {([
        ["system", "跟随系统"],
        ["light", "浅色"],
        ["dark", "深色"],
      ] as const).map(([id, label]) => (
        <button
          key={id}
          type="button"
          className={value === id ? "active" : ""}
          aria-pressed={value === id}
          onClick={() => onChange(id)}
        >
          {value === id ? <Check aria-hidden="true" /> : null}
          {label}
        </button>
      ))}
    </div>
  );
}

export default function AppearanceSettingsPanel() {
  const importInput = useRef<HTMLInputElement>(null);
  const [library, setLibrary] = useState(() => getSkinLibrary());
  const [activeId, setActiveId] = useState(() => readActiveSkinId());
  const [preferences, setPreferences] = useState<AppPreferences>(() => readAppPreferences());
  const [draft, setDraft] = useState<SkinDefinition | null>(null);
  const [editingMode, setEditingMode] = useState<"light" | "dark">(
    document.documentElement.dataset.theme === "dark" ? "dark" : "light",
  );

  const activeSkin = useMemo(
    () => library.find((skin) => skin.id === activeId) ?? resolveSkin(activeId),
    [activeId, library],
  );
  const editPalette = draft?.[editingMode];
  const contrast = draft && editPalette
    ? contrastRatio(editPalette.background, editPalette.foreground)
    : 21;

  const refreshLibrary = () => setLibrary(getSkinLibrary());

  const updatePreferences = (patch: Partial<AppPreferences>) => {
    const next = { ...preferences, ...patch };
    setPreferences(next);
    writeAppPreferences(next);
    applyAppPreferences(next);
    setEditingMode(document.documentElement.dataset.theme === "dark" ? "dark" : "light");
  };

  const applyLibrarySkin = (skin: SkinDefinition) => {
    writeActiveSkinId(skin.id);
    setActiveId(skin.id);
    setDraft(null);
    applySkin(skin);
    notify(`已应用皮肤「${skin.name}」`);
  };

  const startEditing = (source: SkinDefinition) => {
    const editable = source.builtIn ? cloneSkin(source, `${source.name} 自定义`) : structuredClone(source);
    setDraft(editable);
    applySkin(editable);
  };

  const updateDraft = (updater: (current: SkinDefinition) => SkinDefinition) => {
    setDraft((current) => {
      if (!current) return current;
      const next = updater(current);
      applySkin(next);
      return next;
    });
  };

  const updatePalette = (key: PaletteKey, value: string) => {
    updateDraft((current) => ({
      ...current,
      [editingMode]: {
        ...current[editingMode],
        [key]: value,
      },
    }));
  };

  const cancelDraft = () => {
    setDraft(null);
    applySkin(resolveSkin(activeId));
  };

  const saveDraft = () => {
    if (!draft) return;
    const saved = saveCustomSkin(draft);
    writeActiveSkinId(saved.id);
    applySkin(saved);
    setActiveId(saved.id);
    setDraft(null);
    refreshLibrary();
    notify(`皮肤「${saved.name}」已保存`);
  };

  const duplicateActive = () => {
    const copy = cloneSkin(activeSkin, `${activeSkin.name} 副本`);
    const saved = saveCustomSkin(copy);
    refreshLibrary();
    setDraft(saved);
    applySkin(saved);
    notify("已创建可编辑的皮肤副本");
  };

  const removeSkin = (skin: SkinDefinition) => {
    if (skin.builtIn) return;
    if (!window.confirm(`删除自定义皮肤「${skin.name}」？`)) return;
    deleteCustomSkin(skin.id);
    const nextId = skin.id === activeId ? "lifetrace" : activeId;
    setActiveId(nextId);
    const next = resolveSkin(nextId);
    applySkin(next);
    refreshLibrary();
    if (draft?.id === skin.id) setDraft(null);
    notify("自定义皮肤已删除");
  };

  return (
    <article className="hx-panel lt-appearance-panel">
      <PanelHead kicker="外观" title="皮肤与界面" />
      <div className="hx-panel-body lt-appearance-body">
        <section className="lt-appearance-section">
          <div className="lt-appearance-section-head">
            <div>
              <h3>显示模式</h3>
              <p>浅色、深色或跟随系统。皮肤会同时保存两套颜色。</p>
            </div>
          </div>
          <ThemeModeSelector
            value={preferences.theme}
            onChange={(theme) => updatePreferences({ theme })}
          />
        </section>

        <section className="lt-appearance-section">
          <div className="lt-appearance-section-head">
            <div>
              <h3>皮肤库</h3>
              <p>官方皮肤可直接应用；编辑官方皮肤时会自动创建个人副本。</p>
            </div>
            <div className="lt-appearance-actions">
              <button className="hx-btn secondary" type="button" onClick={duplicateActive}>
                <Copy aria-hidden="true" /> 复制当前
              </button>
              <button className="hx-btn secondary" type="button" onClick={() => importInput.current?.click()}>
                <Import aria-hidden="true" /> 导入
              </button>
              <button
                className="hx-btn secondary"
                type="button"
                onClick={() => downloadText(exportSkinDocument(activeSkin), `${activeSkin.id}.lifetrace-skin.json`)}
              >
                <Download aria-hidden="true" /> 导出
              </button>
              <input
                ref={importInput}
                hidden
                type="file"
                accept=".json,.lifetrace-skin,application/json"
                onChange={async (event) => {
                  const file = event.target.files?.[0];
                  event.currentTarget.value = "";
                  if (!file) return;
                  try {
                    const imported = importSkinDocument(await file.text());
                    const saved = saveCustomSkin(imported);
                    refreshLibrary();
                    setDraft(saved);
                    applySkin(saved);
                    notify(`已导入「${saved.name}」，可先预览再保存`);
                  } catch (error) {
                    notify(error instanceof Error ? error.message : "皮肤导入失败", "error");
                  }
                }}
              />
            </div>
          </div>

          <div className="lt-skin-library">
            {library.map((skin) => (
              <div
                key={skin.id}
                className={`lt-skin-card${activeId === skin.id ? " active" : ""}`}
              >
                <button type="button" className="lt-skin-card-main" onClick={() => applyLibrarySkin(skin)}>
                  <SkinPreview skin={skin} />
                  <span className="lt-skin-card-meta">
                    <strong>{skin.name}</strong>
                    <small>{skin.builtIn ? "LifeTrace 官方" : "我的皮肤"}</small>
                  </span>
                  {activeId === skin.id ? <Check className="lt-skin-selected" aria-label="当前皮肤" /> : null}
                </button>
                <div className="lt-skin-card-actions">
                  <button type="button" onClick={() => startEditing(skin)} aria-label={`编辑 ${skin.name}`} title="编辑">
                    <Edit3 aria-hidden="true" />
                  </button>
                  {!skin.builtIn ? (
                    <button type="button" onClick={() => removeSkin(skin)} aria-label={`删除 ${skin.name}`} title="删除">
                      <Trash2 aria-hidden="true" />
                    </button>
                  ) : null}
                </div>
              </div>
            ))}
            <button type="button" className="lt-new-skin" onClick={duplicateActive}>
              <Plus aria-hidden="true" />
              <span>新建自定义皮肤</span>
            </button>
          </div>
        </section>

        {draft ? (
          <section className="lt-skin-studio" aria-label="皮肤编辑器">
            <div className="lt-studio-head">
              <div>
                <span className="lt-studio-icon"><Palette aria-hidden="true" /></span>
                <div>
                  <h3>Skin Studio</h3>
                  <p>实时预览 · 受控 Token · 无自定义 CSS</p>
                </div>
              </div>
              <div className="lt-appearance-actions">
                <button className="hx-btn secondary" type="button" onClick={cancelDraft}>
                  <RotateCcw aria-hidden="true" /> 取消
                </button>
                <button className="hx-btn primary" type="button" onClick={saveDraft}>
                  <Save aria-hidden="true" /> 保存皮肤
                </button>
              </div>
            </div>

            <div className="lt-studio-grid">
              <div className="lt-studio-controls">
                <label className="lt-field">
                  <span>皮肤名称</span>
                  <input
                    type="text"
                    maxLength={48}
                    value={draft.name}
                    onChange={(event) => updateDraft((current) => ({ ...current, name: event.target.value }))}
                  />
                </label>

                <div className="lt-editor-mode">
                  <span>编辑颜色</span>
                  <div className="lt-appearance-segment small">
                    <button type="button" className={editingMode === "light" ? "active" : ""} onClick={() => setEditingMode("light")}>浅色</button>
                    <button type="button" className={editingMode === "dark" ? "active" : ""} onClick={() => setEditingMode("dark")}>深色</button>
                  </div>
                </div>

                <div className="lt-color-grid">
                  {COLOR_FIELDS.map(({ key, label }) => (
                    <label key={key} className="lt-color-control">
                      <span>{label}</span>
                      <span className="lt-color-input-wrap">
                        <input
                          type="color"
                          value={editPalette?.[key] ?? "#000000"}
                          onChange={(event) => updatePalette(key, event.target.value)}
                        />
                        <code>{editPalette?.[key]}</code>
                      </span>
                    </label>
                  ))}
                </div>

                {contrast < 4.5 ? (
                  <p className="lt-contrast-warning" role="alert">
                    当前正文与背景对比度为 {contrast.toFixed(2)}:1，建议至少达到 4.5:1。
                  </p>
                ) : null}

                <div className="lt-slider-grid">
                  <label className="lt-range-control">
                    <span><b>圆角</b><output>{Math.round(draft.visual.roundness)}</output></span>
                    <input
                      type="range"
                      min="0"
                      max="100"
                      value={draft.visual.roundness}
                      onChange={(event) => updateDraft((current) => ({ ...current, visual: { ...current.visual, roundness: Number(event.target.value) } }))}
                    />
                  </label>
                  <label className="lt-range-control">
                    <span><b>阴影</b><output>{Math.round(draft.visual.shadowStrength)}</output></span>
                    <input
                      type="range"
                      min="0"
                      max="100"
                      value={draft.visual.shadowStrength}
                      onChange={(event) => updateDraft((current) => ({ ...current, visual: { ...current.visual, shadowStrength: Number(event.target.value) } }))}
                    />
                  </label>
                  <label className="lt-range-control">
                    <span><b>面板不透明度</b><output>{Math.round(draft.visual.surfaceOpacity)}%</output></span>
                    <input
                      type="range"
                      min="72"
                      max="100"
                      value={draft.visual.surfaceOpacity}
                      onChange={(event) => updateDraft((current) => ({ ...current, visual: { ...current.visual, surfaceOpacity: Number(event.target.value) } }))}
                    />
                  </label>
                </div>

                <div className="lt-background-editor">
                  <div className="lt-editor-mode">
                    <span>应用背景</span>
                    <div className="lt-appearance-segment small">
                      <button
                        type="button"
                        className={draft.background.type === "solid" ? "active" : ""}
                        onClick={() => updateDraft((current) => ({ ...current, background: { ...current.background, type: "solid" } }))}
                      >纯色</button>
                      <button
                        type="button"
                        className={draft.background.type === "gradient" ? "active" : ""}
                        onClick={() => updateDraft((current) => ({ ...current, background: { ...current.background, type: "gradient" } }))}
                      >渐变</button>
                    </div>
                  </div>
                  {draft.background.type === "gradient" ? (
                    <div className="lt-gradient-controls">
                      <label className="lt-color-control">
                        <span>起始色</span>
                        <input type="color" value={draft.background.from} onChange={(event) => updateDraft((current) => ({ ...current, background: { ...current.background, from: event.target.value } }))} />
                      </label>
                      <label className="lt-color-control">
                        <span>结束色</span>
                        <input type="color" value={draft.background.to} onChange={(event) => updateDraft((current) => ({ ...current, background: { ...current.background, to: event.target.value } }))} />
                      </label>
                      <label className="lt-range-control">
                        <span><b>角度</b><output>{Math.round(draft.background.angle)}°</output></span>
                        <input type="range" min="0" max="360" value={draft.background.angle} onChange={(event) => updateDraft((current) => ({ ...current, background: { ...current.background, angle: Number(event.target.value) } }))} />
                      </label>
                      <label className="lt-range-control">
                        <span><b>强度</b><output>{Math.round(draft.background.opacity)}%</output></span>
                        <input type="range" min="0" max="70" value={draft.background.opacity} onChange={(event) => updateDraft((current) => ({ ...current, background: { ...current.background, opacity: Number(event.target.value) } }))} />
                      </label>
                    </div>
                  ) : null}
                </div>
              </div>

              <div className="lt-studio-preview-panel">
                <span className="lt-preview-label">实时组件预览</span>
                <div className="lt-live-preview">
                  <aside>
                    <strong>LT</strong>
                    <span className="active">总览</span>
                    <span>坚持</span>
                    <span>记账</span>
                  </aside>
                  <main>
                    <header><b>今日总览</b><i /></header>
                    <section>
                      <strong>保持自己的节奏</strong>
                      <p>皮肤只改变视觉 Token，不改变业务布局。</p>
                      <div className="lt-preview-buttons">
                        <button type="button">主要操作</button>
                        <button type="button">次要操作</button>
                      </div>
                    </section>
                    <section className="lt-preview-list">
                      <span><i className="success" />练习英语 <small>已完成</small></span>
                      <span><i className="warning" />今日复盘 <small>待完成</small></span>
                    </section>
                  </main>
                </div>
                <div className="lt-preview-swatches">
                  {COLOR_FIELDS.slice(5).map(({ key, label }) => (
                    <span key={key} title={label} style={{ background: editPalette?.[key] }} />
                  ))}
                </div>
              </div>
            </div>
          </section>
        ) : null}
      </div>
    </article>
  );
}
