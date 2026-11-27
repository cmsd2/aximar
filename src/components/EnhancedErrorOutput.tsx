import { useState } from "react";
import { useLingui } from "@lingui/react/macro";
import type { ErrorInfo } from "../types/maxima";
import { errorText } from "../lib/messages";
import { useNotebookStore } from "../store/notebookStore";

interface EnhancedErrorOutputProps {
  error: string;
  errorInfo: ErrorInfo | null;
  /** Maxima's message in the user's language; `error` keeps the English */
  errorTranslated?: string | null;
  cellId: string;
}

export function EnhancedErrorOutput({
  error,
  errorInfo,
  errorTranslated,
  cellId,
}: EnhancedErrorOutputProps) {
  const { t, i18n } = useLingui();
  const [showRaw, setShowRaw] = useState(false);
  const updateCellInput = useNotebookStore((s) => s.updateCellInput);

  if (!errorInfo) {
    return (
      <div className="error-output">
        {errorTranslated ? (
          <>
            <pre>{errorTranslated}</pre>
            <button
              className="enhanced-error-toggle"
              onClick={() => setShowRaw(!showRaw)}
            >
              {showRaw
                ? t({
                    message: "Hide original",
                    comment: "Button: hides Maxima's English error message, shown below its translation",
                  })
                : t({
                    message: "Show original",
                    comment: "Button: shows Maxima's English error message below its translation",
                  })}
            </button>
            {showRaw && <pre className="enhanced-error-raw">{error}</pre>}
          </>
        ) : (
          <pre>{error}</pre>
        )}
      </div>
    );
  }

  return (
    <div className="enhanced-error">
      <div className="enhanced-error-title">{errorText(i18n, errorInfo, "title")}</div>
      {errorTranslated && (
        <pre className="enhanced-error-translated">{errorTranslated}</pre>
      )}
      <div className="enhanced-error-explanation">
        {errorText(i18n, errorInfo, "explanation")}
      </div>

      {errorInfo.suggestion && (
        <div className="enhanced-error-suggestion">
          {errorText(i18n, errorInfo, "suggestion")}
        </div>
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
        {showRaw
          ? t({
              message: "Hide raw error",
              comment: "Button: hides the error exactly as Maxima printed it",
            })
          : t({
              message: "Show raw error",
              comment: "Button: shows the error exactly as Maxima printed it, below Aximar's explanation",
            })}
      </button>
      {showRaw && <pre className="enhanced-error-raw">{error}</pre>}
    </div>
  );
}
