import { useRef, useState } from "react";
import { useUiStyleStore } from "@/src/stores/useUiStyleStore";
import { parseThemeFile } from "@/src/services/themeFiles";

export default function StyleSettingsPanel() {
  const uiStyle = useUiStyleStore(state => state.uiStyle);
  const customThemes = useUiStyleStore(state => state.customThemes);
  const activeThemeId = useUiStyleStore(state => state.activeThemeId);
  const setUiStyle = useUiStyleStore(state => state.setUiStyle);
  const importTheme = useUiStyleStore(state => state.importTheme);
  const removeTheme = useUiStyleStore(state => state.removeTheme);
  const enableTheme = useUiStyleStore(state => state.enableTheme);
  const input = useRef<HTMLInputElement>(null);
  const [message, setMessage] = useState("");

  const handleFile = async (file: File) => {
    try {
      const theme = parseThemeFile(await file.text(), file.name);
      importTheme(theme);
      setMessage(`已导入主题：${theme.name}`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "主题导入失败");
    }
  };

  const builtIns: { id: "classic" | "editorial"; label: string; desc: string }[] = [
    { id: "classic", label: "经典", desc: "原有外观" },
    { id: "editorial", label: "编辑 · 纸质", desc: "纸面质感，衬线标题" },
  ];

  return (
    <article className="hx-panel">
      <header className="hx-panel-head">
        <div>
          <span>界面风格</span>
          <h2>风格与主题</h2>
        </div>
      </header>
      <div className="hx-panel-body">
        <div className="st-style-row">
          {builtIns.map(option => {
            const active = uiStyle === option.id && !activeThemeId;
            return (
              <button type="button" key={option.id} className={active ? "active" : ""} onClick={() => setUiStyle(option.id)}>
                <strong>{option.label}</strong>
                <small>{option.desc}</small>
              </button>
            );
          })}
        </div>
        <p className="st-hint">自定义主题基于"编辑 · 纸质"风格生效，可覆盖配色与背景图。</p>
        <div className="st-theme-actions">
          <button className="hx-btn secondary" onClick={() => input.current?.click()}>导入主题文件</button>
          <input
            ref={input}
            hidden
            type="file"
            accept=".json,.css,application/json,text/css"
            onChange={event => {
              const file = event.target.files?.[0];
              if (file) void handleFile(file);
              event.target.value = "";
            }}
          />
        </div>
        {customThemes.length > 0 && (
          <div className="st-theme-list">
            {customThemes.map(theme => {
              const active = activeThemeId === theme.id;
              return (
                <div className={`st-theme-row ${active ? "active" : ""}`} key={theme.id}>
                  <div className="st-theme-copy">
                    <strong>{theme.name}</strong>
                    <small>{theme.source === "json" ? "JSON 主题" : "CSS 主题"} · {theme.background ? "含背景图" : "仅颜色"}</small>
                  </div>
                  <div className="st-theme-row-actions">
                    <button type="button" className="hx-btn secondary" onClick={() => enableTheme(active ? null : theme.id)}>
                      {active ? "停用" : "启用"}
                    </button>
                    <button type="button" className="hx-btn secondary" onClick={() => removeTheme(theme.id)}>删除</button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
        {message && <p className="st-message">{message}</p>}
      </div>
    </article>
  );
}
