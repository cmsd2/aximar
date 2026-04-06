import { useRef, useEffect, useMemo } from "react";
import Plotly from "plotly.js-dist-min";

interface PlotlyChartProps {
  plotData: string;
}

export function PlotlyChart({ plotData }: PlotlyChartProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  const spec = useMemo(() => {
    try {
      return JSON.parse(plotData) as { data: Plotly.Data[]; layout?: Partial<Plotly.Layout> };
    } catch {
      console.warn("[PlotlyChart] Failed to parse plot data");
      return null;
    }
  }, [plotData]);

  useEffect(() => {
    if (!containerRef.current || !spec) return;

    const computedStyle = getComputedStyle(document.documentElement);
    const textColor = computedStyle.getPropertyValue("--text-primary").trim() || "#e0e0e0";
    const gridColor = computedStyle.getPropertyValue("--border-color").trim() || "#333";

    // If the spec provides explicit width/height, use fixed sizing;
    // otherwise let Plotly auto-size to the container.
    const hasFixedSize = spec.layout?.width || spec.layout?.height;

    const layout: Partial<Plotly.Layout> = {
      ...spec.layout,
      autosize: !hasFixedSize,
      margin: { l: 50, r: 30, t: 40, b: 50 },
      paper_bgcolor: "transparent",
      plot_bgcolor: "transparent",
      font: { color: textColor },
      xaxis: {
        ...(spec.layout?.xaxis as object),
        gridcolor: gridColor,
        zerolinecolor: gridColor,
      },
      yaxis: {
        ...(spec.layout?.yaxis as object),
        gridcolor: gridColor,
        zerolinecolor: gridColor,
      },
    };

    const config: Partial<Plotly.Config> = {
      responsive: !hasFixedSize,
      displayModeBar: true,
      displaylogo: false,
      modeBarButtonsToRemove: ["sendDataToCloud", "toImage", "lasso2d", "select2d"],
    };

    Plotly.newPlot(containerRef.current, spec.data, layout, config);

    // Resize Plotly charts for print: the print CSS changes the container
    // dimensions, so we need to tell Plotly to re-fit.
    const el = containerRef.current;
    const resizePlot = () => {
      if (!el) return;
      // Use matchMedia to get the actual container width under print layout
      const isPrint = window.matchMedia("print").matches;
      if (isPrint) {
        // Force Plotly to use the container's print-constrained size
        const rect = el.getBoundingClientRect();
        Plotly.relayout(el, { width: rect.width, height: Math.min(rect.height || 400, 500) });
      } else {
        // Restore autosize for screen
        if (hasFixedSize) {
          Plotly.relayout(el, {
            width: spec.layout?.width ?? undefined,
            height: spec.layout?.height ?? undefined,
          });
        } else {
          Plotly.relayout(el, { autosize: true, width: undefined, height: undefined });
          Plotly.Plots.resize(el);
        }
      }
    };

    window.addEventListener("beforeprint", resizePlot);
    window.addEventListener("afterprint", resizePlot);

    return () => {
      window.removeEventListener("beforeprint", resizePlot);
      window.removeEventListener("afterprint", resizePlot);
      Plotly.purge(el);
    };
  }, [spec]);

  if (!spec) {
    return <div className="plot-error">Failed to parse plot data</div>;
  }

  return <div ref={containerRef} className="plotly-output" />;
}
