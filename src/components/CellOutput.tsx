import { useState, useCallback, useRef, useMemo } from "react";
import { useLingui } from "@lingui/react/macro";
import type { Cell, CellOutput as CellOutputType } from "../types/notebook";
import { KatexOutput } from "./KatexOutput";
import { TextWithNotes } from "./TextWithNotes";
import { EnhancedErrorOutput } from "./EnhancedErrorOutput";
import { ErrorBoundary } from "./ErrorBoundary";
import { sanitizeSvg } from "../lib/sanitize-svg";
import { PlotlyChart } from "./PlotlyChart";
import { savePlotSvg, savePlotlyImage, savePlotlyJson } from "../lib/plot-save";

/** Which parts of an output there are to show */
function outputParts(output: CellOutputType) {
  const ok = !output.isError;
  const hasLatex = ok && output.latex !== null && output.latex !== "";
  const hasText = ok && output.textOutput !== "";
  const hasPlot = ok && !!output.plotSvg;
  const hasPlotData = ok && !!output.plotData;
  const hasImage = ok && !!output.imagePng;
  // A result with only its one-line form, as from a Jupyter file
  const hasPlain = ok && !hasLatex && !hasPlot && !hasPlotData && !hasImage && !!output.textPlain;
  return { hasLatex, hasText, hasPlot, hasPlotData, hasImage, hasPlain };
}

/**
 * The output a cell shows, or null for none: a cell ending in `$` with
 * nothing to show has none; one ending in `;` keeps its "(no output)"
 */
export function shownOutput(cell: Cell): CellOutputType | null {
  if (!cell.output) return null;
  return cell.silent && outputIsEmpty(cell.output) ? null : cell.output;
}

/** Whether an output has nothing to show: no result, text, plot or error */
export function outputIsEmpty(output: CellOutputType): boolean {
  if (output.isError && output.error) return false;
  return !Object.values(outputParts(output)).some(Boolean);
}

interface CellOutputProps {
  output: CellOutputType;
  cellId: string;
}

export function CellOutput({ output, cellId }: CellOutputProps) {
  const { t, i18n } = useLingui();
  const outputRef = useRef<HTMLDivElement>(null);
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

  const { hasLatex, hasText, hasPlot, hasPlotData, hasImage, hasPlain } = outputParts(output);
  // The Copy button copies the result's Maxima expression when there is one
  const copyText = output.textPlain || output.textOutput;

  // A data URL, not a blob URL: a blob URL made here and revoked in an
  // effect's cleanup broke under StrictMode, which runs the cleanup on its
  // trial unmount but keeps the memoized URL, so the image loaded nothing.
  const plotSrc = useMemo(() => {
    if (!hasPlot) return "";
    const sanitized = sanitizeSvg(output.plotSvg!);
    if (!sanitized) return "";
    return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(sanitized)}`;
  }, [hasPlot, output.plotSvg]);

  if (output.isError && output.error) {
    return (
      <EnhancedErrorOutput
        error={output.error}
        errorInfo={output.errorInfo}
        errorTranslated={output.errorTranslated ?? null}
        cellId={cellId}
      />
    );
  }

  /** The Plotly chart drawn in this output */
  const chart = () => outputRef.current?.querySelector<HTMLElement>(".plotly-output") ?? null;
  const saveTitle = (format: string) =>
    t({
      message: `Save the plot as ${format}`,
      comment: "Tooltip of a button above a plot: saves it to a file. format is SVG, PNG or JSON",
    });

  return (
    <div className="cell-output" ref={outputRef}>
      {(hasLatex || hasText || hasPlot || hasPlotData || hasPlain) && (
        <div className="copy-actions">
          {hasLatex && (
            <button
              className={`copy-btn${copiedBtn === "tex" ? " copied" : ""}`}
              onClick={() => handleCopy(output.latex!, "tex")}
              title={t({
                message: "Copy LaTeX source",
                comment: "Tooltip of the TeX button above a result: copies the result's LaTeX to the clipboard",
              })}
            >
              {copiedBtn === "tex"
                ? t({
                    message: "Copied!",
                    comment: "Shown for a moment on a copy button above a result after it copied. max-length:8",
                  })
                : "TeX"}
            </button>
          )}
          {(hasText || hasPlain || (hasLatex && !!output.textPlain)) && (
            <button
              className={`copy-btn${copiedBtn === "text" ? " copied" : ""}`}
              onClick={() => handleCopy(copyText, "text")}
              title={t({
                message: "Copy Maxima expression",
                comment: "Tooltip of the Copy button above a result: copies the result as Maxima code",
              })}
            >
              {copiedBtn === "text"
                ? t({
                    message: "Copied!",
                    comment: "Shown for a moment on a copy button above a result after it copied. max-length:8",
                  })
                : t({
                    message: "Copy",
                    comment: "Button above a result: copies it to the clipboard as Maxima code. max-length:8",
                  })}
            </button>
          )}
          {/* Saving a plot: the gnuplot SVG as it is, or a Plotly chart as drawn */}
          {hasPlot && (
            <button className="copy-btn" title={saveTitle("SVG")} onClick={() => void savePlotSvg(i18n, output.plotSvg!)}>
              SVG
            </button>
          )}
          {hasPlotData && (
            <>
              <button
                className="copy-btn"
                title={saveTitle("SVG")}
                onClick={() => {
                  const el = chart();
                  if (el) void savePlotlyImage(i18n, el, "svg");
                }}
              >
                SVG
              </button>
              <button
                className="copy-btn"
                title={saveTitle("PNG")}
                onClick={() => {
                  const el = chart();
                  if (el) void savePlotlyImage(i18n, el, "png");
                }}
              >
                PNG
              </button>
              <button className="copy-btn" title={saveTitle("JSON")} onClick={() => void savePlotlyJson(output.plotData!)}>
                JSON
              </button>
            </>
          )}
        </div>
      )}
      {/* Print text and notes first, in the order Maxima printed them */}
      {hasText && <TextWithNotes text={output.textOutput} notes={output.notes ?? []} />}
      {hasPlotData && (
        <ErrorBoundary
          label="PlotlyChart"
          fallback={(err) => {
            const reason = err.message;
            return (
              <div className="plot-error">
                {t({
                  message: `Failed to render plot: ${reason}`,
                  comment: "Shown in place of a chart that could not be drawn. reason is the error text from the chart library, in English",
                })}
              </div>
            );
          }}
        >
          <PlotlyChart plotData={output.plotData!} />
        </ErrorBoundary>
      )}
      {hasPlot && plotSrc && (
        <div className="plot-output">
          <img src={plotSrc}alt={t({ message: "Plot output", comment: "Alt text of a plot image in a cell's output" })} />
        </div>
      )}
      {hasImage && (
        <div className="plot-output">
          <img src={`data:image/png;base64,${output.imagePng}`} alt={t({ message: "Image output", comment: "Alt text of an image in a cell's output" })} style={{ maxWidth: "100%" }} />
        </div>
      )}
      {hasLatex && <KatexOutput latex={output.latex!} />}
      {hasPlain && <div className="text-output">{output.textPlain}</div>}
      {!hasLatex && !hasText && !hasPlot && !hasPlotData && !hasImage && !hasPlain && (
        <span className="text-output empty-output">
          {t({
            message: "(no output)",
            comment: "Shown in a cell's output area when the cell ran and produced nothing to show",
          })}
        </span>
      )}
    </div>
  );
}
