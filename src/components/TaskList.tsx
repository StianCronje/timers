import { useState } from "react";
import { api } from "../api";
import { formatDuration, formatDurationLong } from "../format";
import { useNow, useTasks } from "../hooks";
import type { TaskWithStats } from "../types";

interface Props {
  onOpen: (id: number) => void;
}

export function TaskList({ onOpen }: Props) {
  const [showArchived, setShowArchived] = useState(false);
  const { tasks } = useTasks(showArchived);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const now = useNow(1000);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = name.trim();
    if (!trimmed) return;
    await api.createTask(trimmed, description.trim() || undefined);
    setName("");
    setDescription("");
  };

  return (
    <div className="task-list">
      <form className="task-create" onSubmit={submit}>
        <input
          placeholder="Task name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          autoFocus
        />
        <input
          placeholder="Description (optional)"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
        <button type="submit" disabled={!name.trim()}>
          Add task
        </button>
      </form>

      <div className="list-toolbar">
        <label>
          <input
            type="checkbox"
            checked={showArchived}
            onChange={(e) => setShowArchived(e.target.checked)}
          />
          Show archived
        </label>
      </div>

      {tasks.length === 0 ? (
        <p className="empty">No tasks yet. Create one above.</p>
      ) : (
        <ul className="tasks">
          {tasks.map((t) => (
            <TaskRow key={t.id} task={t} now={now} onOpen={onOpen} />
          ))}
        </ul>
      )}
    </div>
  );
}

function TaskRow({
  task,
  now,
  onOpen,
}: {
  task: TaskWithStats;
  now: number;
  onOpen: (id: number) => void;
}) {
  const isActive = task.active_session !== null;
  const liveElapsed = isActive ? now - task.active_session!.start_time : 0;

  return (
    <li className={`task-row ${task.archived ? "archived" : ""} ${isActive ? "active" : ""}`}>
      <div className="task-main" onClick={() => onOpen(task.id)}>
        <div className="task-name">{task.name}</div>
        {task.description && <div className="task-desc">{task.description}</div>}
        <div className="task-total">
          Total: <strong>{formatDurationLong(task.total_seconds)}</strong>
          {isActive && <span className="live-pill">+ {formatDuration(liveElapsed)}</span>}
        </div>
      </div>
      <div className="task-actions">
        {isActive ? (
          <button className="stop-btn" onClick={() => api.stopTimer()}>
            Pause
          </button>
        ) : (
          <button className="start-btn" onClick={() => api.startTimer(task.id)}>
            {task.total_seconds > 0 ? "Resume" : "Start"}
          </button>
        )}
        <button className="open-btn" onClick={() => onOpen(task.id)}>
          Details
        </button>
        <button
          className="archive-btn"
          onClick={() => api.setTaskArchived(task.id, !task.archived)}
          title={task.archived ? "Unarchive" : "Archive"}
        >
          {task.archived ? "Unarchive" : "Archive"}
        </button>
      </div>
    </li>
  );
}
