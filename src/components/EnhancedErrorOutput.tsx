import { useState } from "react";
import type { ErrorInfo } from "../types/maxima";
import { useNotebookStore } from "../store/notebookStore";

interface EnhancedErrorOutputProps {
  error: string;
  errorInfo: ErrorInfo | null;
  cellId: string;
}

export function EnhancedErrorOutput({
  error,
  errorInfo,
  cellId,
}: EnhancedErrorOutputProps) {
  const [showRaw, setShowRaw] = useState(false);
  const updateCellInput = useNotebookStore((s) => s.updateCellInput);

  if (!errorInfo) {
    return (
      <div className="error-output">
        <pre>{error}</pre>
      </div>
    );
  }

  return (
    <div className="enhanced-error">
      <div className="enhanced-error-title">{errorInfo.title}</div>
      <div className="enhanced-error-explanation">{errorInfo.explanation}</div>

      {errorInfo.suggestion && (
        <div className="enhanced-error-suggestion">{errorInfo.suggestion}</div>
      )}

      {errorInfo.did_you_mean.length > 0 && (
        <div className="enhanced-error-dym">
          Did you mean:{" "}
          {errorInfo.did_you_mean.map((name, i) => (
            <span key={name}>
              {i > 0 && ", "}
              <button
                className="dym-link"
                onClick={() => updateCellInput(cellId, name + "()")}
              >
                {name}
              </button>
            </span>
          ))}
          ?
        </div>
      )}

      {errorInfo.correct_signatures.length > 0 && (
        <div className="enhanced-error-sigs">
          <span className="enhanced-error-sigs-label">Correct usage:</span>
          {errorInfo.correct_signatures.map((sig) => (
            <code key={sig} className="enhanced-error-sig">
              {sig}
            </code>
          ))}
        </div>
      )}

      {errorInfo.example && (
        <div className="enhanced-error-example">
          Example:{" "}
          <code
            className="enhanced-error-example-code"
            onClick={() => updateCellInput(cellId, errorInfo.example!)}
            title="Click to use this example"
          >
            {errorInfo.example}
          </code>
        </div>
      )}

      <button
        className="enhanced-error-toggle"
        onClick={() => setShowRaw(!showRaw)}
      >
        {showRaw ? "Hide" : "Show"} raw error
      </button>
      {showRaw && <pre className="enhanced-error-raw">{error}</pre>}
    </div>
  );
}
