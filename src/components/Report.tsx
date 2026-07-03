import { useEffect, useMemo, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "../api";
import {
  addDays,
  formatDate,
  formatDurationLong,
  startOfMonthUnix,
  startOfWeekUnix,
} from "../format";
import type { ReportEntry } from "../types";

type RangeKind = "week" | "month";

interface Range {
  kind: RangeKind;
  offset: number; // 0 = current, -1 = previous
}

function computeRange(r: Range): { start: number; end: number; label: string } {
  const today = new Date();
  if (r.kind === "week") {
    const startThis = startOfWeekUnix(today);
    const start = addDays(startThis, r.offset * 7);
    const end = addDays(start, 7);
    return {
      start,
      end,
      label: `Week of ${formatDate(start)}`,
    };
  }
  const d = new Date(today.getFullYear(), today.getMonth() + r.offset, 1);
  const start = startOfMonthUnix(d);
  const next = new Date(d.getFullYear(), d.getMonth() + 1, 1);
  const end = Math.floor(next.getTime() / 1000);
  const label = d.toLocaleString(undefined, { month: "long", year: "numeric" });
  return { start, end, label };
}

export function Report() {
  const [range, setRange] = useState<Range>({ kind: "week", offset: 0 });
  const [entries, setEntries] = useState<ReportEntry[]>([]);
  const computed = useMemo(() => computeRange(range), [range]);

  const refresh = () => {
    api.report(computed.start, computed.end).then(setEntries).catch(() => setEntries([]));
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
    const tick = setInterval(refresh, 30_000);
    return () => {
      unlistenA?.();
      unlistenB?.();
      clearInterval(tick);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [computed.start, computed.end]);

  const total = entries.reduce((sum, e) => sum + e.total_seconds, 0);
  const max = entries.reduce((m, e) => Math.max(m, e.total_seconds), 0);

  return (
    <div className="report">
      <div className="report-controls">
        <div className="kind-toggle">
          <button
            className={range.kind === "week" ? "active" : ""}
            onClick={() => setRange({ kind: "week", offset: 0 })}
          >
            Week
          </button>
          <button
            className={range.kind === "month" ? "active" : ""}
            onClick={() => setRange({ kind: "month", offset: 0 })}
          >
            Month
          </button>
        </div>
        <div className="nav-toggle">
          <button onClick={() => setRange({ ...range, offset: range.offset - 1 })}>
            ← Prev
          </button>
          <span className="range-label">{computed.label}</span>
          <button
            onClick={() => setRange({ ...range, offset: range.offset + 1 })}
            disabled={range.offset >= 0}
          >
            Next →
          </button>
        </div>
      </div>

      <div className="report-summary">
        Total: <strong>{formatDurationLong(total)}</strong>
        <span className="muted"> across {entries.length} task{entries.length === 1 ? "" : "s"}</span>
      </div>

      {entries.length === 0 ? (
        <p className="empty">No time logged in this range.</p>
      ) : (
        <ul className="report-list">
          {entries.map((e) => (
            <li key={e.task_id} className="report-row">
              <div className="report-row-name">{e.task_name}</div>
              <div className="report-row-bar">
                <div
                  className="report-bar"
                  style={{ width: max > 0 ? `${(e.total_seconds / max) * 100}%` : "0%" }}
                />
              </div>
              <div className="report-row-total">{formatDurationLong(e.total_seconds)}</div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
