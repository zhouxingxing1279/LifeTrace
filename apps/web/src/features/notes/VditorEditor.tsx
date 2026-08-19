import { useEffect, useRef } from "react";
import Vditor from "vditor";
import "vditor/dist/index.css";

const VDITOR_CDN = "https://unpkg.com/vditor@3.11.3";

const VDITOR_I18N = new Proxy<Record<string, string>>({
  bold: "粗体", both: "分屏预览", check: "任务列表", close: "关闭", code: "代码块",
  "code-theme": "代码主题", "content-theme": "内容主题", copied: "已复制", copy: "复制",
  edit: "编辑", "edit-mode": "编辑模式", emoji: "表情", export: "导出", fullscreen: "全屏",
  headings: "标题", help: "帮助", indent: "缩进", info: "信息", "inline-code": "行内代码",
  "insert-after": "在后方插入", "insert-before": "在前方插入", instantRendering: "即时渲染",
  italic: "斜体", line: "分隔线", link: "链接", list: "无序列表", more: "更多",
  "ordered-list": "有序列表", outdent: "减少缩进", outline: "大纲", preview: "预览",
  quote: "引用", redo: "重做", splitView: "分屏预览", strike: "删除线", table: "表格",
  undo: "撤销", update: "更新", wysiwyg: "所见即所得", tooltipText: "提示",
}, {
  get(target, key) {
    if (typeof key !== "string") return "";
    return target[key] ?? key;
  },
});

export interface VditorEditorProps {
  value: string;
  onChange(value: string): void;
  onSave?(): void;
}

function currentTheme(): "classic" | "dark" {
  return document.documentElement.dataset.theme === "dark" ? "dark" : "classic";
}

function contentTheme(): "dark" | "ant-design" {
  return currentTheme() === "dark" ? "dark" : "ant-design";
}

export function VditorEditor({ value, onChange, onSave }: VditorEditorProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<Vditor | null>(null);
  const readyRef = useRef(false);
  const valueRef = useRef(value);
  const onChangeRef = useRef(onChange);
  const onSaveRef = useRef(onSave);

  valueRef.current = value;
  onChangeRef.current = onChange;
  onSaveRef.current = onSave;

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    let disposed = false;
    let editor: Vditor;
    editor = new Vditor(host, {
      value: valueRef.current,
      mode: "ir",
      minHeight: 520,
      height: "auto",
      lang: "zh_CN",
      i18n: VDITOR_I18N as never,
      cdn: VDITOR_CDN,
      cache: { enable: false },
      theme: currentTheme(),
      typewriterMode: false,
      tab: "    ",
      counter: { enable: true, type: "text" },
      outline: { enable: false, position: "left" },
      toolbarConfig: { pin: true },
      toolbar: [
        "headings", "bold", "italic", "strike", "link", "|",
        "list", "ordered-list", "check", "quote", "line", "|",
        "code", "inline-code", "table", "|",
        "undo", "redo", "|", "fullscreen", "edit-mode",
        { name: "more", toolbar: ["both", "outline", "preview", "export", "info", "help"] },
      ],
      preview: {
        delay: 250,
        maxWidth: 900,
        mode: "both",
        theme: { current: contentTheme() },
        hljs: { enable: true, lineNumber: true, style: currentTheme() === "dark" ? "dracula" : "github" },
        markdown: {
          sanitize: true,
          footnotes: true,
          toc: true,
          mark: true,
          gfmAutoLink: true,
          codeBlockPreview: true,
          mathBlockPreview: true,
        },
      },
      input(markdown) {
        if (!disposed) onChangeRef.current(markdown);
      },
      keydown(event) {
        if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
          event.preventDefault();
          onSaveRef.current?.();
        }
      },
      after() {
        readyRef.current = true;
        if (disposed) {
          editor.destroy();
          return;
        }
        editorRef.current = editor;
        const latest = valueRef.current;
        if (editor.getValue() !== latest) editor.setValue(latest, true);
      },
    });

    const observer = new MutationObserver(() => {
      if (!readyRef.current || !editorRef.current) return;
      const dark = currentTheme() === "dark";
      editorRef.current.setTheme(dark ? "dark" : "classic", dark ? "dark" : "ant-design", dark ? "dracula" : "github");
    });
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });

    return () => {
      disposed = true;
      observer.disconnect();
      if (readyRef.current) editor.destroy();
      editorRef.current = null;
      readyRef.current = false;
    };
  }, []);

  useEffect(() => {
    const editor = editorRef.current;
    if (!readyRef.current || !editor) return;
    if (editor.getValue() !== value) editor.setValue(value, true);
  }, [value]);

  return <div className="lifetrace-vditor min-w-0 overflow-hidden rounded-md border bg-background" ref={hostRef} data-testid="vditor-editor" />;
}
