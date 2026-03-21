import { useEffect, useRef } from "react";
import { useLogStore } from "../store/logStore";

interface LogPanelProps {
  open: boolean;
}

export function LogPanel({ open }: LogPanelProps) {
  const entries = useLogStore((s) => s.entries);
  const clearLog = useLogStore((s) => s.clearLog);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [entries.length]);

  if (!open) return null;

  return (
    <div className="log-panel">
      <div className="log-panel-header">
        <span className="log-panel-title">LOG</span>
        <button className="log-panel-clear" onClick={clearLog}>
          Clear
        </button>
      </div>
      <div className="log-panel-body">
        {entries.length === 0 && (
          <div className="log-panel-empty">No log entries</div>
        )}
        {entries.map((entry) => (
          <div key={entry.id} className={`log-entry log-entry-${entry.level}`}>
            <span className="log-entry-level">{entry.level}</span>
            <span className="log-entry-source">{entry.source}</span>
            <span className="log-entry-message">{entry.message}</span>
          </div>
        ))}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
