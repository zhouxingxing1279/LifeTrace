import { useState } from "react";
import { NotebookPen } from "lucide-react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import type { WorkoutHistory } from "@/src/types";
import XunjiImportPanel from "@/src/components/XunjiImportPanel";
import MobileUploadControl from "@/src/components/common/MobileUploadControl";
import { EmptyState, PanelHead } from "@/src/components/common";
import { Button } from "@/src/components/ui";

export default function Fitness({
  note,
}: {
  note: (value: WorkoutHistory) => void;
}) {
  const { workoutHistory } = useLifeStore();
  const [referenceTime] = useState(() => Date.now());
  const weekCount = workoutHistory.filter(
    (item) => referenceTime - new Date(item.occurredAt).getTime() < 7 * 86400000,
  ).length;

  return (
    <div className="hx-view">
      <article className="hx-fitness-hero">
        <div>
          <span className="hx-kicker">训练数据</span>
          <h2>导入训练截图，自动整理训练记录</h2>
          <p>电脑端负责解析训练数据并长期保存，已有训练历史会继续保留。</p>
        </div>
        <div>
          <span>本周训练</span>
          <strong>{weekCount} / 4</strong>
          <i className="hx-track">
            <b style={{ width: `${Math.min(100, (weekCount / 4) * 100)}%` }} />
          </i>
        </div>
      </article>

      <MobileUploadControl context="fitness" />
      <XunjiImportPanel />

      <article className="hx-panel hx-history">
        <PanelHead kicker="训练记录" title="训练历史" />
        <div>
          {workoutHistory.slice(0, 10).map((item) => (
            <div className="hx-history-row" key={item.id}>
              <time>
                {new Date(item.occurredAt).toLocaleDateString("zh-CN", {
                  month: "2-digit",
                  day: "2-digit",
                })}
              </time>
              <div>
                <strong>{item.name}</strong>
                <small>
                  {item.exerciseCount} 个动作 · {item.setCount} 组 ·{" "}
                  {Math.max(1, Math.round(item.durationSeconds / 60))} 分钟
                </small>
              </div>
              <Button
                variant="secondary"
                icon={<NotebookPen aria-hidden="true" />}
                onClick={() => note(item)}
              >
                训练复盘
              </Button>
            </div>
          ))}
          {!workoutHistory.length ? (
            <EmptyState
              title="暂无训练记录"
              hint="导入训练截图后，解析结果会出现在这里。"
            />
          ) : null}
        </div>
      </article>
    </div>
  );
}
