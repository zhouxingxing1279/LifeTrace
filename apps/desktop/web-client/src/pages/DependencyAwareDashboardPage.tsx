import { useMemo } from "react";
import { localDate, taskMatchesToday, type CloudState, type JsonEntity } from "../core";
import { Empty, Metric, MetricGrid, PageStack, Panel, entities, text } from "../ui";
import { DashboardPage } from "./DashboardPage";

interface Props { state: CloudState; privacy: boolean; }

function open(task?: JsonEntity): boolean {
  return Boolean(task && task.status !== "done" && task.status !== "cancelled");
}

export function DependencyAwareDashboardPage({ state, privacy }: Props) {
  const tasks = useMemo(() => entities(state, "execution.task"), [state]);
  const dependencies = useMemo(() => entities(state, "execution.task_dependency"), [state]);
  const occurrences = useMemo(() => entities(state, "execution.task_occurrence"), [state]);
  const taskMap = new Map(tasks.map((task) => [task.meta.id, task]));
  const today = localDate();
  const recurringTodayTaskIds = new Set(occurrences.filter((item) => text(item, "occurrenceKey") === today && item.status !== "completed" && item.status !== "skipped").map((item) => text(item, "taskId")));
  const todays = tasks.filter((task) => taskMatchesToday(task, today) || recurringTodayTaskIds.has(task.meta.id)).filter(open);
  const blockersFor = (taskId: string) => dependencies
    .filter((edge) => text(edge, "taskId") === taskId)
    .map((edge) => taskMap.get(text(edge, "dependsOnTaskId")))
    .filter((item): item is JsonEntity => open(item));
  const blocked = todays.map((task) => ({ task, blockers: blockersFor(task.meta.id) })).filter((item) => item.blockers.length > 0);
  const blockedIds = new Set(blocked.map((item) => item.task.meta.id));
  const ready = todays.filter((task) => !blockedIds.has(task.meta.id));

  return <PageStack>
    <MetricGrid>
      <Metric label="今天可直接执行" value={String(ready.length)} detail="前置条件已满足" positive={ready.length > 0} />
      <Metric label="今天被阻塞" value={String(blocked.length)} detail="先处理前置任务，再开始后续任务" />
    </MetricGrid>
    {blocked.length > 0 && <Panel eyebrow="DEPENDENCY-AWARE TODAY" title="今天暂时不能开始的任务">
      <div className="hx-list">
        {blocked.map(({ task, blockers }) => <article className="hx-row" key={task.meta.id}>
          <span className="hx-row-icon">锁</span>
          <div className="hx-row-main"><strong>{text(task, "title")}</strong><small>等待：{blockers.map((item) => text(item, "title")).join("、")}</small></div>
        </article>)}
      </div>
    </Panel>}
    {!todays.length && <Empty title="今天没有执行任务" description="任务安排到今天后，LifeTrace 会自动根据 finish-before-start 依赖区分可执行和被阻塞事项。" />}
    <DashboardPage state={state} privacy={privacy} />
  </PageStack>;
}
