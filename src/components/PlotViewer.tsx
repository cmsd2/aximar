import { PlotlyChart } from "./PlotlyChart";

interface PlotViewerProps {
  plotData: string;
}

export function PlotViewer({ plotData }: PlotViewerProps) {
  return (
    <div className="plot-viewer">
      <PlotlyChart plotData={plotData} />
    </div>
  );
}
