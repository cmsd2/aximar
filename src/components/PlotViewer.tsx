import { useCallback } from "react";
import { useLingui } from "@lingui/react/macro";
import { PlotlyChart } from "./PlotlyChart";
import { Button } from "./ui/Button";
import { savePlotlyImage, savePlotlyJson } from "../lib/plot-save";

interface PlotViewerProps {
  plotData: string;
}

export function PlotViewer({ plotData }: PlotViewerProps) {
  const { t, i18n } = useLingui();
  const getPlotEl = useCallback(
    () => document.querySelector(".plot-viewer .plotly-output") as HTMLElement | null,
    [],
  );

  const handleExport = useCallback(
    async (format: "svg" | "png") => {
      const plotEl = getPlotEl();
      if (plotEl) await savePlotlyImage(i18n, plotEl, format);
    },
    [getPlotEl, i18n],
  );

  const handleSaveJson = useCallback(() => {
    void savePlotlyJson(plotData);
  }, [plotData]);

  return (
    <div className="plot-viewer">
      <div className="plot-viewer-toolbar">
        <Button size="sm" onClick={() => handleExport("svg")}>
          {t({
            message: "Export SVG",
            comment: "Button in the plot viewer window: saves the chart as an SVG image. max-length:14",
          })}
        </Button>
        <Button size="sm" onClick={() => handleExport("png")}>
          {t({
            message: "Export PNG",
            comment: "Button in the plot viewer window: saves the chart as a PNG image. max-length:14",
          })}
        </Button>
        <Button size="sm" onClick={handleSaveJson}>
          {t({
            message: "Save JSON",
            comment: "Button in the plot viewer window: saves the chart's Plotly data as a JSON file. max-length:14",
          })}
        </Button>
      </div>
      <PlotlyChart plotData={plotData} />
    </div>
  );
}
