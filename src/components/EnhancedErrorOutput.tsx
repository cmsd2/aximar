import { useState } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import type { ErrorInfo } from "../types/maxima";
import { errorText } from "../lib/messages";
import { useNotebookStore } from "../store/notebookStore";
import { IconAlertCircle, IconBulb } from "./ui/icons";

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

  if (!errorInfo) {
    // No explanation: Maxima's own text, after the error icon
    return (
      <div className="error-output">
        <ErrorIcon />
        <div className="error-output-body">
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
      </div>
    );
  }

  return (
    <div className="enhanced-error">
      <div className="enhanced-error-title">
        <ErrorIcon />
        <span>{errorText(i18n, errorInfo, "title")}</span>
      </div>
      <div className="enhanced-error-explanation">
        {errorText(i18n, errorInfo, "explanation")}
      </div>
      {/* Maxima's own message in the user's language, under Aximar's explanation */}
      {errorTranslated && (
        <pre className="enhanced-error-translated">{errorTranslated}</pre>
      )}

      {errorInfo.suggestion && (
        <div className="enhanced-error-suggestion">
          <IconBulb className="enhanced-error-bulb" size={15} stroke={1.75} aria-hidden="true" />
          <span>{errorText(i18n, errorInfo, "suggestion")}</span>
        </div>
      )}

      {errorInfo.did_you_mean.length > 0 && (
        <div className="enhanced-error-dym">
          <DidYouMean names={errorInfo.did_you_mean} cellId={cellId} />
        </div>
      )}

      {errorInfo.correct_signatures.length > 0 && (
        <div className="enhanced-error-sigs">
          <span className="enhanced-error-sigs-label">
            {t({
              message: "Correct usage:",
              comment: "Label under an error explanation, before the right ways to call the Maxima function",
            })}
          </span>
          {errorInfo.correct_signatures.map((sig) => (
            <code key={sig} className="enhanced-error-sig">
              {sig}
            </code>
          ))}
        </div>
      )}

      {errorInfo.example && (
        <div className="enhanced-error-example">
          <ErrorExample example={errorInfo.example} cellId={cellId} />
        </div>
      )}

      <button
        className="enhanced-error-toggle"
        onClick={() => setShowRaw(!showRaw)}
      >
        {errorTranslated
          ? showRaw
            ? t({
                message: "Hide original",
                comment: "Button: hides Maxima's English error message, shown below its translation",
              })
            : t({
                message: "Show original",
                comment: "Button: shows Maxima's English error message below its translation",
              })
          : showRaw
            ? t({
                message: "Hide Maxima's error",
                comment: "Button under an error explanation: hides the error exactly as Maxima printed it",
              })
            : t({
                message: "Show Maxima's error",
                comment: "Button under an error explanation: shows the error exactly as Maxima printed it, below Aximar's explanation",
              })}
      </button>
      {showRaw && <pre className="enhanced-error-raw">{error}</pre>}
    </div>
  );
}

/** "Did you mean: a, b?", each name a button that puts the call in the cell */
function DidYouMean({ names: nameList, cellId }: { names: string[]; cellId: string }) {
  const updateCellInput = useNotebookStore((s) => s.updateCellInput);
  const names = (
    <>
      {nameList.map((name, i) => (
        <span key={name}>
          {i > 0 && ", "}
          <button className="dym-link" onClick={() => updateCellInput(cellId, name + "()")}>
            {name}
          </button>
        </span>
      ))}
    </>
  );
  return (
    <Trans comment="Under an error about an unknown function. names is a comma-separated list of Maxima function names, each a button that puts the name in the cell">
      Did you mean: {names}?
    </Trans>
  );
}

/** "Example: <code>", the code a button that puts it in the cell */
function ErrorExample({ example: exampleText, cellId }: { example: string; cellId: string }) {
  const { t } = useLingui();
  const updateCellInput = useNotebookStore((s) => s.updateCellInput);
  const example = (
    <code
      className="enhanced-error-example-code"
      onClick={() => updateCellInput(cellId, exampleText)}
      title={t({
        message: "Click to use this example",
        comment: "Tooltip of the example Maxima code under an error explanation: clicking it replaces the cell's input",
      })}
    >
      {exampleText}
    </code>
  );
  return (
    <Trans comment="Under an error explanation. example is a line of Maxima code, which the user can click to put in the cell">
      Example: {example}
    </Trans>
  );
}

/** The red mark before an error: the one place the output itself is red */
function ErrorIcon() {
  return <IconAlertCircle className="error-icon" size={16} stroke={1.75} aria-hidden="true" />;
}
