import { Search } from "lucide-react";
import { useMemo, useState } from "react";
import { searchEntities, type CloudState } from "../core";
import { navigate, type Route } from "../navigation";
import { Empty, Panel } from "../ui";

export function SearchPage({ state }: { state: CloudState }) {
  const [query, setQuery] = useState("");
  const normalized = query.trim();
  const hits = useMemo(() => searchEntities(state, normalized), [state, normalized]);

  return <div className="lt-search-page lt-page-stack">
    <Panel eyebrow="SEARCH" title="全局搜索">
      <div className="hx-search-box lt-search-input">
        <Search />
        <input
          autoFocus
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="搜索坚持、训练、账单、笔记、文章、生词和复盘"
          aria-label="搜索 LifeTrace"
        />
        {normalized && <kbd>{hits.length}</kbd>}
      </div>
      <p className="lt-search-hint">输入关键词后会跨模块搜索云端实体，结果按最近更新时间排序。</p>
    </Panel>

    <div className="hx-search-results lt-search-results" aria-live="polite">
      {hits.map((hit) => <button key={`${hit.entityType}-${hit.id}`} onClick={() => navigate(hit.route as Route)}>
        <span>{friendlyType(hit.entityType)}</span>
        <div><strong>{hit.title}</strong><p>{hit.subtitle.slice(0, 180)}</p><small>{new Date(hit.updatedAt).toLocaleString("zh-CN")}</small></div>
      </button>)}
      {normalized && !hits.length && <Empty title="没有搜索结果" description="尝试名称、商户、笔记内容或其他更短的关键词。" />}
      {!normalized && <div className="lt-search-empty"><Search /><strong>从一个关键词开始</strong><p>无需先选择模块，LifeTrace 会直接在你的云端数据中查找。</p></div>}
    </div>
  </div>;
}

function friendlyType(value: string): string {
  if (value.startsWith("habit.")) return "坚持";
  if (value.startsWith("workout.")) return "训练";
  if (value.startsWith("finance.")) return "财务";
  if (value.startsWith("note.")) return "笔记";
  if (value.startsWith("english.")) return "英语";
  if (value.startsWith("review.")) return "复盘";
  return value;
}
