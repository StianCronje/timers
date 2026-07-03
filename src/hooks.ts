import { useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "./api";
import type { ActiveTimerInfo, TaskWithStats } from "./types";
import { nowSeconds } from "./format";

export function useNow(intervalMs = 1000): number {
  const [now, setNow] = useState(nowSeconds());
  useEffect(() => {
    const id = setInterval(() => setNow(nowSeconds()), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);
  return now;
}

export function useActiveTimer(): ActiveTimerInfo | null {
  const [active, setActive] = useState<ActiveTimerInfo | null>(null);
  useEffect(() => {
    let mounted = true;
    api.getActiveTimer().then((a) => {
      if (mounted) setActive(a);
    });
    let unlisten: UnlistenFn | undefined;
    listen<ActiveTimerInfo | null>("timer:active-changed", (e) => {
      setActive(e.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);
  return active;
}

export function useTasks(includeArchived = false): {
  tasks: TaskWithStats[];
  refresh: () => void;
} {
  const [tasks, setTasks] = useState<TaskWithStats[]>([]);
  const refresh = () => {
    api.listTasks(includeArchived).then(setTasks).catch(() => {});
  };
  useEffect(() => {
    refresh();
    let unlisten: UnlistenFn | undefined;
    listen("tasks:changed", () => refresh()).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [includeArchived]);
  return { tasks, refresh };
}
