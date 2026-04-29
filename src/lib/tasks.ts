import { invoke } from "@tauri-apps/api/core";

const TASK_HISTORY_KEY = "clearc.taskHistory.v1";
const MAX_TASKS = 30;

export type TaskStatus = "running" | "completed" | "failed" | "cancelled";

export type TaskRecord = {
  id: string;
  label: string;
  source: string;
  status: TaskStatus;
  startedAt: number;
  finishedAt?: number;
  detail?: string;
};

export function startTask(label: string, source: string): string {
  const id = `${source}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  const task: TaskRecord = {
    id,
    label,
    source,
    status: "running",
    startedAt: Date.now(),
  };
  writeTasks([task, ...readTasks()].slice(0, MAX_TASKS));
  return id;
}

export function completeTask(id: string, detail?: string) {
  finishTask(id, "completed", detail);
}

export function failTask(id: string, detail?: string) {
  finishTask(id, "failed", detail);
}

export function cancelTask(id: string, detail?: string) {
  finishTask(id, "cancelled", detail);
}

export function requestTaskCancel(task: string) {
  return invoke<void>("request_task_cancel", { task });
}

export function readTasks(): TaskRecord[] {
  try {
    const raw = window.localStorage.getItem(TASK_HISTORY_KEY);
    if (!raw) {
      return [];
    }
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function clearCompletedTasks() {
  writeTasks(readTasks().filter((task) => task.status === "running"));
}

function finishTask(id: string, status: TaskStatus, detail?: string) {
  writeTasks(
    readTasks().map((task) =>
      task.id === id
        ? {
            ...task,
            status,
            detail,
            finishedAt: Date.now(),
          }
        : task
    )
  );
}

function writeTasks(tasks: TaskRecord[]) {
  window.localStorage.setItem(TASK_HISTORY_KEY, JSON.stringify(tasks.slice(0, MAX_TASKS)));
}
