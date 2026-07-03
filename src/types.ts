export interface Task {
  id: number;
  name: string;
  description: string | null;
  created_at: number;
  archived: boolean;
}

export interface Session {
  id: number;
  task_id: number;
  start_time: number;
  end_time: number | null;
}

export interface TaskWithStats extends Task {
  total_seconds: number;
  active_session: Session | null;
}

export interface TaskDetail {
  task: Task;
  sessions: Session[];
  total_seconds: number;
  active_session: Session | null;
}

export interface ReportEntry {
  task_id: number;
  task_name: string;
  total_seconds: number;
}

export interface ActiveTimerInfo {
  task: Task;
  session: Session;
}
