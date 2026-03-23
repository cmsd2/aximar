export type LogLevel = "info" | "warning" | "error";

export interface LogEntry {
  id: string;
  timestamp: number;
  level: LogLevel;
  message: string;
  source: string;
}

export interface RawOutputEntry {
  id: string;
  line: string;
  stream: "stdin" | "stdout" | "stderr";
  timestamp: number;
}

export type LogTab = "app" | "maxima";
