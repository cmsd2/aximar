import { useState, useCallback, useRef, useMemo } from "react";
import type { CellOutput as CellOutputType } from "../types/notebook";
import { KatexOutput } from "./KatexOutput";
import { EnhancedErrorOutput } from "./EnhancedErrorOutput";
import { sanitizeSvg } from "../lib/sanitize-svg";

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

  const hasLatex = !output.isError && output.latex !== null && output.latex !== "";
  const hasText = !output.isError && output.textOutput !== "";
  const hasPlot = !output.isError && output.plotSvg !== null && output.plotSvg !== undefined && output.plotSvg !== "";

  const plotBlobUrl = useMemo(() => {
    if (!hasPlot) return "";
    const sanitized = sanitizeSvg(output.plotSvg!);
    if (!sanitized) return "";
    const blob = new Blob([sanitized], { type: "image/svg+xml" });
    return URL.createObjectURL(blob);
  }, [hasPlot, output.plotSvg]);

  // Revoke previous blob URL on unmount or when it changes
  const prevBlobUrl = useRef<string>("");
  if (prevBlobUrl.current && prevBlobUrl.current !== plotBlobUrl) {
    URL.revokeObjectURL(prevBlobUrl.current);
  }
  prevBlobUrl.current = plotBlobUrl;

  if (output.isError && output.error) {
    return (
      <EnhancedErrorOutput
        error={output.error}
        errorInfo={output.errorInfo}
        cellId={cellId}
      />
    );
  }

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
      {hasPlot && plotBlobUrl && (
        <div className="plot-output">
          <img src={plotBlobUrl} alt="Plot output" />
        </div>
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
