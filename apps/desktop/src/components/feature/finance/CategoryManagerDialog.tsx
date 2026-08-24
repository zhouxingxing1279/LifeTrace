import { useState } from "react";
import { Save, Trash2, X } from "lucide-react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import type { FinanceCategory } from "@/src/types";
import { confirmAction } from "@/src/ui/feedback/confirm";
import { notify } from "@/src/ui/feedback/toastBus";

export default function CategoryManagerDialog({ onClose }: { onClose: () => void }) {
  const { categories, saveCategory, archiveCategory } = useLifeStore();
  const [type, setType] = useState<"expense" | "income">("expense");
  const [name, setName] = useState("");
  const [editing, setEditing] = useState<Record<string, string>>({});
  const visible = categories.filter((item) => item.type === type && !item.isArchived);

  const save = async (category?: FinanceCategory) => {
    const nextName = (category ? editing[category.id] ?? category.name : name).trim();
    if (!nextName) return;
    await saveCategory({ ...category, name: nextName, type });
    if (!category) setName("");
    notify(category ? "分类已更新" : "分类已创建");
  };

  return <div className="hx-overlay" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
    <section className="hx-modal finance-category-dialog" role="dialog" aria-modal="true" aria-label="管理账单分类">
      <header><div><span className="hx-kicker">账单设置</span><h2>分类管理</h2></div><button type="button" aria-label="关闭" onClick={onClose}><X /></button></header>
      <div className="finance-category-tabs">
        <button type="button" className={type === "expense" ? "active" : ""} onClick={() => setType("expense")}>支出分类</button>
        <button type="button" className={type === "income" ? "active" : ""} onClick={() => setType("income")}>收入分类</button>
      </div>
      <div className="finance-category-create"><input value={name} onChange={(event) => setName(event.target.value)} placeholder="输入新分类名称" onKeyDown={(event) => { if (event.key === "Enter") void save(); }} /><button type="button" className="hx-btn primary" disabled={!name.trim()} onClick={() => void save()}>新增分类</button></div>
      <div className="finance-category-list">
        {visible.map((item) => <div key={item.id}>
          <input value={editing[item.id] ?? item.name} disabled={item.isSystem} onChange={(event) => setEditing((current) => ({ ...current, [item.id]: event.target.value }))} />
          {!item.isSystem && <><button type="button" aria-label={`保存 ${item.name}`} onClick={() => void save(item)}><Save /></button><button type="button" className="danger" aria-label={`停用 ${item.name}`} onClick={async () => { if (await confirmAction({ title: "停用分类", description: `已有账单仍保留“${item.name}”，新账单将不再显示此分类。`, confirmLabel: "停用" })) await archiveCategory(item.id); }}><Trash2 /></button></>}
          {item.isSystem && <span>系统分类</span>}
        </div>)}
        {!visible.length && <p>还没有自定义分类。创建分类后即可用于记账和筛选。</p>}
      </div>
    </section>
  </div>;
}
