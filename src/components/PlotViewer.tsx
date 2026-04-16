import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import Plotly from "plotly.js-dist-min";
import { PlotlyChart } from "./PlotlyChart";
import { savePlotFile } from "../lib/notebooks-client";

interface PlotViewerProps {
  plotData: string;
}

export function PlotViewer({ plotData }: PlotViewerProps) {
  const getPlotEl = useCallback(
    () => document.querySelector(".plot-viewer .plotly-output") as HTMLElement | null,
    [],
  );

  const handleExport = useCallback(
    async (format: "svg" | "png") => {
      const plotEl = getPlotEl();
      if (!plotEl) return;

      const path = await save({
        defaultPath: `plot.${format}`,
        filters: [{ name: `${format.toUpperCase()} Image`, extensions: [format] }],
      });
      if (!path) return;

      const imgData = await Plotly.toImage(plotEl, {
        format,
        width: plotEl.clientWidth || 800,
        height: plotEl.clientHeight || 500,
      });

      if (format === "svg") {
        const svgContent = imgData.startsWith("data:")
          ? decodeURIComponent(imgData.split(",")[1])
          : imgData;
        await invoke("write_plot_svg", { path, content: svgContent });
      } else {
        const base64 = imgData.split(",")[1];
        const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
        await invoke("write_binary_file", { path, data: Array.from(bytes) });
      }
    },
    [getPlotEl],
  );

  const handleSaveJson = useCallback(() => {
    savePlotFile(plotData, null);
  }, [plotData]);

  return (
    <div className="plot-viewer">
      <div className="plot-viewer-toolbar">
        <button className="suggestion-chip" onClick={() => handleExport("svg")}>
          Export SVG
        </button>
        <button className="suggestion-chip" onClick={() => handleExport("png")}>
          Export PNG
        </button>
        <button className="suggestion-chip" onClick={handleSaveJson}>
          Save JSON
        </button>
      </div>
      <PlotlyChart plotData={plotData} />
    </div>
  );
}
