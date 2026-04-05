import { useState, useCallback, useRef, useMemo, useEffect } from "react";
import type { CellOutput as CellOutputType } from "../types/notebook";
import { KatexOutput } from "./KatexOutput";
import { RichTextOutput } from "./RichTextOutput";
import { EnhancedErrorOutput } from "./EnhancedErrorOutput";
import { sanitizeSvg } from "../lib/sanitize-svg";
import { PlotlyChart } from "./PlotlyChart";

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
  const hasPlotData = !output.isError && output.plotData !== null && output.plotData !== undefined && output.plotData !== "";

  const plotBlobUrl = useMemo(() => {
    if (!hasPlot) return "";
    const sanitized = sanitizeSvg(output.plotSvg!);
    if (!sanitized) return "";
    const blob = new Blob([sanitized], { type: "image/svg+xml" });
    return URL.createObjectURL(blob);
  }, [hasPlot, output.plotSvg]);

  // Revoke blob URL when it changes or on unmount
  useEffect(() => {
    return () => {
      if (plotBlobUrl) {
        URL.revokeObjectURL(plotBlobUrl);
      }
    };
  }, [plotBlobUrl]);

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
      {(hasLatex || hasText || hasPlot || hasPlotData) && (
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
      {hasPlotData && <PlotlyChart plotData={output.plotData!} />}
      {hasPlot && plotBlobUrl && (
        <div className="plot-output">
          <img src={plotBlobUrl} alt="Plot output" />
        </div>
      )}
      {hasText && <RichTextOutput text={output.textOutput} />}
      {hasLatex && <KatexOutput latex={output.latex!} />}
      {!hasLatex && !hasText && !hasPlot && !hasPlotData && (
        <span className="text-output empty-output">(no output)</span>
      )}
    </div>
  );
}
