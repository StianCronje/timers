import { api } from "../api";
import { formatDuration } from "../format";
import { useActiveTimer, useNow } from "../hooks";

export function ActiveBar() {
  const active = useActiveTimer();
  const now = useNow(1000);

  if (!active) {
    return (
      <div className="active-bar idle">
        <span className="active-label">No active timer</span>
      </div>
    );
  }

  const elapsed = now - active.session.start_time;

  return (
    <div className="active-bar running">
      <span className="active-label">Running:</span>
      <span className="active-task">{active.task.name}</span>
      <span className="active-elapsed">{formatDuration(elapsed)}</span>
      <button className="stop-btn" onClick={() => api.stopTimer()}>
        Pause
      </button>
    </div>
  );
}
