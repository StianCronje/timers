import { invoke } from "@tauri-apps/api/core";
import type {
  ActiveTimerInfo,
  ReportEntry,
  Session,
  Task,
  TaskDetail,
  TaskWithStats,
} from "./types";

export const api = {
  createTask: (name: string, description?: string) =>
    invoke<Task>("create_task", { name, description: description ?? null }),
  updateTask: (id: number, name: string, description?: string) =>
    invoke<Task>("update_task", { id, name, description: description ?? null }),
  setTaskArchived: (id: number, archived: boolean) =>
    invoke<void>("set_task_archived", { id, archived }),
  deleteTask: (id: number) => invoke<void>("delete_task", { id }),
  listTasks: (includeArchived = false) =>
    invoke<TaskWithStats[]>("list_tasks", { includeArchived }),
  getTaskDetail: (id: number) => invoke<TaskDetail>("get_task_detail", { id }),
  startTimer: (taskId: number) => invoke<Session>("start_timer", { taskId }),
  stopTimer: () => invoke<Session | null>("stop_timer"),
  getActiveTimer: () => invoke<ActiveTimerInfo | null>("get_active_timer"),
  report: (rangeStart: number, rangeEnd: number) =>
    invoke<ReportEntry[]>("report", { rangeStart, rangeEnd }),
};
