import { useState, useCallback, useRef } from "react";
import type { CellOutput as CellOutputType } from "../types/notebook";
import { KatexOutput } from "./KatexOutput";
import { EnhancedErrorOutput } from "./EnhancedErrorOutput";

interface CellOutputProps {
  output: CellOutputType;
  cellId: string;
}

export function CellOutput({ output, cellId }: CellOutputProps) {
  const [copiedBtn, setCopiedBtn] = useState<"tex" | "text" | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  const handleCopy = useCallback(
    (text: string, btn: "tex" | "text") => {
      navigator.clipboard.writeText(text).then(() => {
        clearTimeout(timerRef.current);
        setCopiedBtn(btn);
        timerRef.current = setTimeout(() => setCopiedBtn(null), 1500);
      });
    },
    [],
  );

  if (output.isError && output.error) {
    return (
      <EnhancedErrorOutput
        error={output.error}
        errorInfo={output.errorInfo}
        cellId={cellId}
      />
    );
  }

  const hasLatex = output.latex !== null && output.latex !== "";
  const hasText = output.textOutput !== "";
  const hasPlot = output.plotSvg !== null && output.plotSvg !== undefined && output.plotSvg !== "";

  return (
    <div className="cell-output">
      {(hasLatex || hasText || hasPlot) && (
        <div className="copy-actions">
          {hasLatex && (
            <button
              className={`copy-btn${copiedBtn === "tex" ? " copied" : ""}`}
              onClick={() => handleCopy(output.latex!, "tex")}
              title="Copy LaTeX source"
            >
              {copiedBtn === "tex" ? "Copied!" : "TeX"}
            </button>
          )}
          {hasText && (
            <button
              className={`copy-btn${copiedBtn === "text" ? " copied" : ""}`}
              onClick={() => handleCopy(output.textOutput, "text")}
              title="Copy Maxima expression"
            >
              {copiedBtn === "text" ? "Copied!" : "Copy"}
            </button>
          )}
        </div>
      )}
      {hasPlot && (
        <div
          className="plot-output"
          dangerouslySetInnerHTML={{ __html: output.plotSvg! }}
        />
      )}
      {hasLatex && <KatexOutput latex={output.latex!} />}
      {hasText && !hasLatex && !hasPlot && (
        <pre className="text-output">{output.textOutput}</pre>
      )}
      {!hasLatex && !hasText && !hasPlot && (
        <span className="text-output empty-output">(no output)</span>
      )}
    </div>
  );
}
