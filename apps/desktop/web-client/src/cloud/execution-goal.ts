import { baseMeta, type JsonEntity } from "./types";

export type ExecutionGoalStatus = "active" | "paused" | "completed" | "cancelled";

export interface ExecutionGoalInput {
  name: string;
  description?: string;
  targetAt?: string | null;
  color?: string | null;
  icon?: string | null;
}

export function createExecutionGoal(userId: string, deviceId: string, input: ExecutionGoalInput): JsonEntity {
  const name = input.name.trim();
  if (!name) throw new Error("请输入目标名称");
  return {
    meta: baseMeta(userId, deviceId),
    name,
    description: input.description?.trim() || null,
    status: "active",
    targetAt: input.targetAt ?? null,
    color: input.color ?? "#49715d",
    icon: input.icon ?? "target",
    sortOrder: 0,
    completedAt: null,
  };
}

export function goalProjectProgress(goalId: string, projects: JsonEntity[], tasks: JsonEntity[]): { projects: number; completedProjects: number; tasks: number; completedTasks: number; rate: number } {
  const ownedProjects = projects.filter((project) => project.goalId === goalId && project.status !== "cancelled");
  const projectIds = new Set(ownedProjects.map((project) => project.meta.id));
  const ownedTasks = tasks.filter((task) => projectIds.has(String(task.projectId ?? "")) && task.status !== "cancelled");
  const completedProjects = ownedProjects.filter((project) => project.status === "completed").length;
  const completedTasks = ownedTasks.filter((task) => task.status === "done").length;
  return {
    projects: ownedProjects.length,
    completedProjects,
    tasks: ownedTasks.length,
    completedTasks,
    rate: ownedTasks.length ? Math.round((completedTasks / ownedTasks.length) * 100) : completedProjects && completedProjects === ownedProjects.length ? 100 : 0,
  };
}

export function setGoalStatus(goal: JsonEntity, status: ExecutionGoalStatus): JsonEntity {
  const now = new Date().toISOString();
  return {
    ...goal,
    status,
    completedAt: status === "completed" ? now : status === "active" || status === "paused" ? null : goal.completedAt,
  };
}
