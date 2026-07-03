import { useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "../api";
import {
  formatDateTime,
  formatDuration,
  formatDurationLong,
} from "../format";
import { useNow } from "../hooks";
import type { TaskDetail as TaskDetailT } from "../types";

interface Props {
  taskId: number;
  onBack: () => void;
}

export function TaskDetail({ taskId, onBack }: Props) {
  const [detail, setDetail] = useState<TaskDetailT | null>(null);
  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const now = useNow(1000);

  const refresh = () => {
    api.getTaskDetail(taskId).then(setDetail).catch(() => setDetail(null));
  };

  useEffect(() => {
    refresh();
    let unlistenA: UnlistenFn | undefined;
    let unlistenB: UnlistenFn | undefined;
    listen("tasks:changed", () => refresh()).then((fn) => {
      unlistenA = fn;
    });
    listen("timer:active-changed", () => refresh()).then((fn) => {
      unlistenB = fn;
    });
    return () => {
      unlistenA?.();
      unlistenB?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [taskId]);

  if (!detail) {
    return (
      <div className="task-detail">
        <button onClick={onBack}>← Back</button>
        <p>Loading…</p>
      </div>
    );
  }

  const isActive = detail.active_session !== null;
  const liveElapsed = isActive ? now - detail.active_session!.start_time : 0;

  const beginEdit = () => {
    setEditName(detail.task.name);
    setEditDescription(detail.task.description ?? "");
    setEditing(true);
  };

  const saveEdit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!editName.trim()) return;
    await api.updateTask(taskId, editName.trim(), editDescription.trim() || undefined);
    setEditing(false);
  };

  const onDelete = async () => {
    if (!confirm(`Delete "${detail.task.name}" and all its time entries?`)) return;
    await api.deleteTask(taskId);
    onBack();
  };

  return (
    <div className="task-detail">
      <button className="back-btn" onClick={onBack}>
        ← Back
      </button>

      {editing ? (
        <form className="task-edit" onSubmit={saveEdit}>
          <input
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
            autoFocus
          />
          <input
            value={editDescription}
            onChange={(e) => setEditDescription(e.target.value)}
            placeholder="Description"
          />
          <button type="submit" disabled={!editName.trim()}>
            Save
          </button>
          <button type="button" onClick={() => setEditing(false)}>
            Cancel
          </button>
        </form>
      ) : (
        <header className="detail-header">
          <h2>{detail.task.name}</h2>
          {detail.task.description && <p className="detail-desc">{detail.task.description}</p>}
          <div className="detail-actions">
            {isActive ? (
              <button className="stop-btn" onClick={() => api.stopTimer()}>
                Pause
              </button>
            ) : (
              <button className="start-btn" onClick={() => api.startTimer(taskId)}>
                {detail.total_seconds > 0 ? "Resume" : "Start"}
              </button>
            )}
            <button onClick={beginEdit}>Edit</button>
            <button onClick={onDelete} className="danger">
              Delete
            </button>
          </div>
        </header>
      )}

      <section className="detail-summary">
        <div>
          <div className="label">Total time</div>
          <div className="value">{formatDurationLong(detail.total_seconds)}</div>
        </div>
        <div>
          <div className="label">Sessions</div>
          <div className="value">{detail.sessions.length}</div>
        </div>
        {isActive && (
          <div>
            <div className="label">Current session</div>
            <div className="value live">{formatDuration(liveElapsed)}</div>
          </div>
        )}
      </section>

      <section className="sessions">
        <h3>Sessions</h3>
        {detail.sessions.length === 0 ? (
          <p className="empty">No sessions yet.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Start</th>
                <th>End</th>
                <th>Duration</th>
              </tr>
            </thead>
            <tbody>
              {detail.sessions.map((s) => {
                const end = s.end_time ?? now;
                const dur = end - s.start_time;
                return (
                  <tr key={s.id} className={s.end_time === null ? "live-row" : ""}>
                    <td>{formatDateTime(s.start_time)}</td>
                    <td>{s.end_time === null ? <em>running</em> : formatDateTime(s.end_time)}</td>
                    <td>{formatDuration(dur)}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}
