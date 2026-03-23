import { useEffect, useRef, useState, useCallback } from "react";
import { useLogStore } from "../store/logStore";
import type { LogTab } from "../types/log";

/** Persistent single-line status bar at the bottom of the app. */
export function StatusBar() {
  const entries = useLogStore((s) => s.entries);
  const unreadCount = useLogStore((s) => s.unreadCount);
  const windowOpen = useLogStore((s) => s.windowOpen);
  const openWindow = useLogStore((s) => s.openWindow);

  const latest = entries.length > 0 ? entries[entries.length - 1] : null;

  const formatTime = (ts: number) => {
    const d = new Date(ts);
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  };

  return (
    <div className="status-bar" onClick={openWindow}>
      {latest ? (
        <>
          <span className={`status-bar-level status-bar-level-${latest.level}`}>
            {latest.level}
          </span>
          <span className="status-bar-message">{latest.message}</span>
          <span className="status-bar-time">{formatTime(latest.timestamp)}</span>
        </>
      ) : (
        <span className="status-bar-message status-bar-empty">No log entries</span>
      )}
      {!windowOpen && unreadCount > 0 && (
        <span className="status-bar-badge">{unreadCount}</span>
      )}
    </div>
  );
}

const DEFAULT_HEIGHT = 200;
const MIN_HEIGHT = 80;
const MAX_HEIGHT_RATIO = 0.6;

/** Expandable log window with tabs for App Log and Maxima Output. */
export function LogWindow() {
  const windowOpen = useLogStore((s) => s.windowOpen);
  const activeTab = useLogStore((s) => s.activeTab);
  const setActiveTab = useLogStore((s) => s.setActiveTab);
  const closeWindow = useLogStore((s) => s.closeWindow);
  const entries = useLogStore((s) => s.entries);
  const rawOutput = useLogStore((s) => s.rawOutput);
  const clearLog = useLogStore((s) => s.clearLog);
  const clearRawOutput = useLogStore((s) => s.clearRawOutput);
  const bottomRef = useRef<HTMLDivElement>(null);
  const [height, setHeight] = useState(DEFAULT_HEIGHT);
  const dragging = useRef(false);
  const startY = useRef(0);
  const startH = useRef(0);

  const onPointerDown = useCallback((e: React.PointerEvent) => {
    dragging.current = true;
    startY.current = e.clientY;
    startH.current = height;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }, [height]);

  const onPointerMove = useCallback((e: React.PointerEvent) => {
    if (!dragging.current) return;
    const delta = startY.current - e.clientY;
    const maxH = window.innerHeight * MAX_HEIGHT_RATIO;
    setHeight(Math.max(MIN_HEIGHT, Math.min(maxH, startH.current + delta)));
  }, []);

  const onPointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  const scrollDeps = activeTab === "app" ? entries.length : rawOutput.length;
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [scrollDeps]);

  if (!windowOpen) return null;

  const tabs: { key: LogTab; label: string }[] = [
    { key: "app", label: "App Log" },
    { key: "maxima", label: "Maxima Output" },
  ];

  const formatTime = (ts: number) => {
    const d = new Date(ts);
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit", fractionalSecondDigits: 3 } as Intl.DateTimeFormatOptions);
  };

  return (
    <div className="log-window" style={{ height }}>
      <div
        className="log-window-resize-handle"
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
      />
      <div className="log-window-header">
        <div className="log-window-tabs">
          {tabs.map((tab) => (
            <button
              key={tab.key}
              className={`log-window-tab${activeTab === tab.key ? " log-window-tab-active" : ""}`}
              onClick={() => setActiveTab(tab.key)}
            >
              {tab.label}
            </button>
          ))}
        </div>
        <div className="log-window-actions">
          <button
            className="log-window-clear"
            onClick={activeTab === "app" ? clearLog : clearRawOutput}
          >
            Clear
          </button>
          <button className="log-window-close" onClick={closeWindow}>
            &times;
          </button>
        </div>
      </div>
      <div className="log-window-body">
        {activeTab === "app" && (
          <>
            {entries.length === 0 && (
              <div className="log-window-empty">No log entries</div>
            )}
            {entries.map((entry) => (
              <div key={entry.id} className={`log-entry log-entry-${entry.level}`}>
                <span className="log-entry-time">{formatTime(entry.timestamp)}</span>
                <span className="log-entry-level">{entry.level}</span>
                <span className="log-entry-source">{entry.source}</span>
                <span className="log-entry-message">{entry.message}</span>
              </div>
            ))}
          </>
        )}
        {activeTab === "maxima" && (
          <>
            {rawOutput.length === 0 && (
              <div className="log-window-empty">No Maxima output</div>
            )}
            {rawOutput.map((entry) => (
              <div key={entry.id} className={`log-raw log-raw-${entry.stream}`}>
                <span className="log-raw-time">{formatTime(entry.timestamp)}</span>
                <span className="log-raw-stream">{entry.stream === "stdin" ? ">" : entry.stream === "stderr" ? "!" : " "}</span>
                <span className="log-raw-line">{entry.line}</span>
              </div>
            ))}
          </>
        )}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
