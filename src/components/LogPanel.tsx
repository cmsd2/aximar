import { useEffect, useMemo, useRef, useState, useCallback } from "react";
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import { useLogStore } from "../store/logStore";
import type {
  EnvelopeEntry,
  LogEntry,
  LogTab,
  RawOutputEntry,
} from "../types/log";

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

type ParsedEnvelope = Record<string, unknown>;

function parseRaw(raw: string): ParsedEnvelope | null {
  try {
    const v = JSON.parse(raw);
    return v && typeof v === "object" ? (v as ParsedEnvelope) : null;
  } catch {
    return null;
  }
}

function ellipsize(s: string, n: number): string {
  return s.length <= n ? s : `${s.slice(0, n - 1)}…`;
}

/** One-line salient summary for the row's collapsed view. */
function summarize(env: ParsedEnvelope | null, kind: string): string {
  if (!env) return "";
  const s = (k: string) => (typeof env[k] === "string" ? (env[k] as string) : undefined);
  const n = (k: string) => (typeof env[k] === "number" ? (env[k] as number) : undefined);
  switch (kind) {
    case "eval_begin":
      return s("eval_id") ?? "";
    case "eval_end": {
      const status = s("status") ?? "";
      const dur = n("duration_ms");
      return dur !== undefined ? `${status} (${dur}ms)` : status;
    }
    case "eval_result": {
      const label = s("output_label");
      const mime = env["mime_bundle"];
      const mimes =
        mime && typeof mime === "object"
          ? Object.keys(mime as Record<string, unknown>).join(", ")
          : "";
      return [label, mimes].filter(Boolean).join("  ");
    }
    case "error": {
      const ek = s("kind") ?? "";
      const msg = (s("message") ?? "").split("\n")[0];
      return [ek, ellipsize(msg, 80)].filter(Boolean).join("  ");
    }
    case "display": {
      const mime = env["mime_bundle"];
      return mime && typeof mime === "object"
        ? Object.keys(mime as Record<string, unknown>).join(", ")
        : "";
    }
    case "vars": {
      const vs = env["vars"];
      if (Array.isArray(vs)) {
        const head = (vs as unknown[]).slice(0, 4).join(", ");
        return vs.length > 4 ? `${head}, … (${vs.length})` : head;
      }
      return "";
    }
    case "output": {
      const stream = s("stream") ?? "";
      const text = (s("text") ?? "").replace(/\n+$/, "");
      return [stream, ellipsize(text, 80)].filter(Boolean).join("  ");
    }
    default:
      return "";
  }
}

function EnvelopeRow({ entry }: { entry: EnvelopeEntry }) {
  const [expanded, setExpanded] = useState(false);
  const parsed = useMemo(() => parseRaw(entry.rawLine), [entry.rawLine]);
  const isError = entry.parseError !== null;
  const kindLabel = entry.kind ?? "parse_error";
  const summary = isError
    ? ellipsize(entry.parseError ?? "", 100)
    : summarize(parsed, kindLabel);
  const pretty = useMemo(
    () => (parsed ? JSON.stringify(parsed, null, 2) : entry.rawLine),
    [parsed, entry.rawLine],
  );
  return (
    <div className={`log-envelope log-envelope-${kindLabel}${isError ? " log-envelope-bad" : ""}`}>
      <div
        className="log-envelope-line"
        onClick={() => setExpanded((v) => !v)}
        role="button"
        tabIndex={0}
      >
        <span className="log-envelope-toggle">{expanded ? "▾" : "▸"}</span>
        <span className="log-envelope-time">{formatTime(entry.timestampMs)}</span>
        <span className="log-envelope-kind">{kindLabel}</span>
        <span className="log-envelope-summary">{summary}</span>
      </div>
      {expanded && <pre className="log-envelope-body">{pretty}</pre>}
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
  const envelopes = useLogStore((s) => s.envelopes);
  const clearLog = useLogStore((s) => s.clearLog);
  const clearRawOutput = useLogStore((s) => s.clearRawOutput);
  const clearEnvelopes = useLogStore((s) => s.clearEnvelopes);
  const [height, setHeight] = useState(DEFAULT_HEIGHT);
  const dragging = useRef(false);
  const startY = useRef(0);
  const startH = useRef(0);

  const appVirtuosoRef = useRef<VirtuosoHandle>(null);
  const rawVirtuosoRef = useRef<VirtuosoHandle>(null);
  const envelopeVirtuosoRef = useRef<VirtuosoHandle>(null);

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
    const ref =
      activeTab === "app"
        ? appVirtuosoRef
        : activeTab === "maxima"
          ? rawVirtuosoRef
          : envelopeVirtuosoRef;
    ref.current?.scrollToIndex({ index: "LAST", behavior: "smooth" });
  }, [
    activeTab === "app"
      ? entries.length
      : activeTab === "maxima"
        ? rawOutput.length
        : envelopes.length,
    atBottom,
    activeTab,
  ]);

  if (!windowOpen) return null;

  const tabs: { key: LogTab; label: string }[] = [
    { key: "app", label: "App Log" },
    { key: "maxima", label: "Maxima Output" },
    { key: "events", label: "Events" },
  ];

  const clearForActiveTab =
    activeTab === "app"
      ? clearLog
      : activeTab === "maxima"
        ? clearRawOutput
        : clearEnvelopes;

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
          <button className="log-window-clear" onClick={clearForActiveTab}>
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
        {activeTab === "events" && (
          envelopes.length === 0 ? (
            <div className="log-window-empty">No envelope frames</div>
          ) : (
            <Virtuoso
              ref={envelopeVirtuosoRef}
              data={envelopes}
              atBottomStateChange={setAtBottom}
              itemContent={(_index, entry) => <EnvelopeRow entry={entry} />}
              followOutput="smooth"
            />
          )
        )}
      </div>
    </div>
  );
}
