import { useEffect } from "react";
import { EditorContent, useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Link from "@tiptap/extension-link";
import TaskList from "@tiptap/extension-task-list";
import TaskItem from "@tiptap/extension-task-item";
import Placeholder from "@tiptap/extension-placeholder";

export interface EditorValue {
  html: string;
  text: string;
  json: unknown;
}

export function RichTextEditor({ value, onChange }: { value: string; onChange: (value: EditorValue) => void }) {
  const editor = useEditor({
    extensions: [
      StarterKit,
      Link.configure({ openOnClick: false, autolink: true }),
      TaskList,
      TaskItem.configure({ nested: true }),
      Placeholder.configure({ placeholder: "开始记录……" }),
    ],
    content: value || "<p></p>",
    immediatelyRender: false,
    onUpdate: ({ editor: current }) => onChange({ html: current.getHTML(), text: current.getText(), json: current.getJSON() }),
  });

  useEffect(() => {
    if (editor && value !== editor.getHTML()) editor.commands.setContent(value || "<p></p>", { emitUpdate: false });
  }, [editor, value]);

  if (!editor) return <div className="editor-loading">正在加载编辑器…</div>;

  const toggleLink = () => {
    const previous = editor.getAttributes("link").href as string | undefined;
    const href = window.prompt("链接地址", previous ?? "https://");
    if (href === null) return;
    if (!href) editor.chain().focus().extendMarkRange("link").unsetLink().run();
    else editor.chain().focus().extendMarkRange("link").setLink({ href }).run();
  };

  return (
    <div className="rich-editor">
      <div className="editor-toolbar" aria-label="富文本工具栏">
        <button type="button" className={editor.isActive("bold") ? "active" : ""} onClick={() => editor.chain().focus().toggleBold().run()}>粗体</button>
        <button type="button" className={editor.isActive("italic") ? "active" : ""} onClick={() => editor.chain().focus().toggleItalic().run()}>斜体</button>
        <button type="button" className={editor.isActive("heading", { level: 2 }) ? "active" : ""} onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}>标题</button>
        <button type="button" className={editor.isActive("bulletList") ? "active" : ""} onClick={() => editor.chain().focus().toggleBulletList().run()}>列表</button>
        <button type="button" className={editor.isActive("taskList") ? "active" : ""} onClick={() => editor.chain().focus().toggleTaskList().run()}>任务</button>
        <button type="button" className={editor.isActive("blockquote") ? "active" : ""} onClick={() => editor.chain().focus().toggleBlockquote().run()}>引用</button>
        <button type="button" className={editor.isActive("link") ? "active" : ""} onClick={toggleLink}>链接</button>
        <button type="button" onClick={() => editor.chain().focus().undo().run()}>撤销</button>
        <button type="button" onClick={() => editor.chain().focus().redo().run()}>重做</button>
      </div>
      <EditorContent editor={editor} />
    </div>
  );
}
