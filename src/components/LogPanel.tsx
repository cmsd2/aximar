import { useEffect, useRef, useState, useCallback } from "react";
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import { useLogStore } from "../store/logStore";
import type { LogEntry, RawOutputEntry, LogTab } from "../types/log";

const formatTime = (ts: number) => {
  const d = new Date(ts);
  return d.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    fractionalSecondDigits: 3,
  } as Intl.DateTimeFormatOptions);
};

function AppLogRow({ entry }: { entry: LogEntry }) {
  return (
    <div className={`log-entry log-entry-${entry.level}`}>
      <span className="log-entry-time">{formatTime(entry.timestamp)}</span>
      <span className="log-entry-level">{entry.level}</span>
      <span className="log-entry-source">{entry.source}</span>
      <span className="log-entry-message">{entry.message}</span>
    </div>
  );
}

function RawOutputRow({ entry }: { entry: RawOutputEntry }) {
  return (
    <div className={`log-raw log-raw-${entry.stream}`}>
      <span className="log-raw-time">{formatTime(entry.timestamp)}</span>
      <span className="log-raw-stream">
        {entry.stream === "stdin" ? ">" : entry.stream === "stderr" ? "!" : " "}
      </span>
      <span className="log-raw-line">{entry.line}</span>
    </div>
  );
}

/** Persistent single-line status bar at the bottom of the app. */
export function StatusBar() {
  const entries = useLogStore((s) => s.entries);
  const unreadCount = useLogStore((s) => s.unreadCount);
  const windowOpen = useLogStore((s) => s.windowOpen);
  const openWindow = useLogStore((s) => s.openWindow);

  const latest = entries.length > 0 ? entries[entries.length - 1] : null;

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
  const [height, setHeight] = useState(DEFAULT_HEIGHT);
  const dragging = useRef(false);
  const startY = useRef(0);
  const startH = useRef(0);

  const appVirtuosoRef = useRef<VirtuosoHandle>(null);
  const rawVirtuosoRef = useRef<VirtuosoHandle>(null);

  const onPointerDown = useCallback(
    (e: React.PointerEvent) => {
      dragging.current = true;
      startY.current = e.clientY;
      startH.current = height;
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    },
    [height],
  );

  const onPointerMove = useCallback((e: React.PointerEvent) => {
    if (!dragging.current) return;
    const delta = startY.current - e.clientY;
    const maxH = window.innerHeight * MAX_HEIGHT_RATIO;
    setHeight(Math.max(MIN_HEIGHT, Math.min(maxH, startH.current + delta)));
  }, []);

  const onPointerUp = useCallback(() => {
    dragging.current = false;
  }, []);

  // Auto-follow: scroll to bottom when new entries arrive
  const [atBottom, setAtBottom] = useState(true);

  useEffect(() => {
    if (!atBottom) return;
    const ref = activeTab === "app" ? appVirtuosoRef : rawVirtuosoRef;
    ref.current?.scrollToIndex({ index: "LAST", behavior: "smooth" });
  }, [
    activeTab === "app" ? entries.length : rawOutput.length,
    atBottom,
    activeTab,
  ]);

  if (!windowOpen) return null;

  const tabs: { key: LogTab; label: string }[] = [
    { key: "app", label: "App Log" },
    { key: "maxima", label: "Maxima Output" },
  ];

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
          entries.length === 0 ? (
            <div className="log-window-empty">No log entries</div>
          ) : (
            <Virtuoso
              ref={appVirtuosoRef}
              data={entries}
              atBottomStateChange={setAtBottom}
              itemContent={(_index, entry) => <AppLogRow entry={entry} />}
              followOutput="smooth"
            />
          )
        )}
        {activeTab === "maxima" && (
          rawOutput.length === 0 ? (
            <div className="log-window-empty">No Maxima output</div>
          ) : (
            <Virtuoso
              ref={rawVirtuosoRef}
              data={rawOutput}
              atBottomStateChange={setAtBottom}
              itemContent={(_index, entry) => <RawOutputRow entry={entry} />}
              followOutput="smooth"
            />
          )
        )}
      </div>
    </div>
  );
}
