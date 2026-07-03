import { useState } from "react";
import { ActiveBar } from "./components/ActiveBar";
import { TaskList } from "./components/TaskList";
import { TaskDetail } from "./components/TaskDetail";
import { Report } from "./components/Report";
import "./App.css";

type View =
  | { kind: "tasks" }
  | { kind: "detail"; id: number }
  | { kind: "report" };

function App() {
  const [view, setView] = useState<View>({ kind: "tasks" });

  return (
    <main className="app">
      <ActiveBar />

      <nav className="tabs">
        <button
          className={view.kind === "tasks" || view.kind === "detail" ? "active" : ""}
          onClick={() => setView({ kind: "tasks" })}
        >
          Tasks
        </button>
        <button
          className={view.kind === "report" ? "active" : ""}
          onClick={() => setView({ kind: "report" })}
        >
          Reports
        </button>
      </nav>

      <div className="content">
        {view.kind === "tasks" && (
          <TaskList onOpen={(id) => setView({ kind: "detail", id })} />
        )}
        {view.kind === "detail" && (
          <TaskDetail
            taskId={view.id}
            onBack={() => setView({ kind: "tasks" })}
          />
        )}
        {view.kind === "report" && <Report />}
      </div>
    </main>
  );
}

export default App;
