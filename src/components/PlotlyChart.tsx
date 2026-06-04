import { useRef, useEffect, useMemo, useState, useCallback } from "react";
import Plotly from "plotly.js-dist-min";
import {
  setSignalAndReplot,
  type ReactiveBlock,
  type ReactiveSignalMeta,
} from "../lib/reactive-client";

interface PlotlyChartProps {
  plotData: string;
}

interface PlotlySpec {
  data: Plotly.Data[];
  layout?: Partial<Plotly.Layout>;
  frames?: Partial<Plotly.Frame>[];
  _reactive?: ReactiveBlock;
}

export function PlotlyChart({ plotData }: PlotlyChartProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  const spec = useMemo<PlotlySpec | null>(() => {
    try {
      return JSON.parse(plotData) as PlotlySpec;
    } catch {
      console.warn("[PlotlyChart] Failed to parse plot data");
      return null;
    }
  }, [plotData]);

  const reactive = spec?._reactive;

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

    Plotly.newPlot(containerRef.current, spec.data, layout, config).then(() => {
      // Animated figures (ax-plots `animate(...)`) carry a top-level
      // `frames` array plus layout.sliders / layout.updatemenus.  The
      // slider steps reference frames by name; addFrames registers them
      // so the play button and slider scrub actually animate.
      if (containerRef.current && spec.frames && spec.frames.length > 0) {
        Plotly.addFrames(containerRef.current, spec.frames);
      }
    });

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

  return (
    <div className="plotly-chart">
      <div ref={containerRef} className="plotly-output" />
      {reactive && reactive.signals.length > 0 && (
        <ReactiveControls containerRef={containerRef} reactive={reactive} />
      )}
    </div>
  );
}

interface ReactiveControlsProps {
  containerRef: React.RefObject<HTMLDivElement | null>;
  reactive: ReactiveBlock;
}

/**
 * Live UI controls for signals embedded in a Plotly figure.  Each slider
 * holds local state for instant visual feedback; updates are sent to the
 * Maxima kernel with a "trailing latest" strategy — at most one invoke
 * is in flight per view, and any newer drag value supersedes pending ones.
 */
function ReactiveControls({ containerRef, reactive }: ReactiveControlsProps) {
  const [values, setValues] = useState<Record<string, number>>(() => {
    const initial: Record<string, number> = {};
    for (const sig of reactive.signals) initial[sig.name] = sig.value;
    return initial;
  });

  // When the registered view changes (different cell re-run), reset state
  // to the freshly-parsed signal metadata.
  useEffect(() => {
    const next: Record<string, number> = {};
    for (const sig of reactive.signals) next[sig.name] = sig.value;
    setValues(next);
  }, [reactive.view_id, reactive.signals]);

  const inflightRef = useRef(false);
  const pendingRef = useRef<{ name: string; value: number } | null>(null);

  const dispatch = useCallback(
    async (name: string, value: number) => {
      if (inflightRef.current) {
        pendingRef.current = { name, value };
        return;
      }
      inflightRef.current = true;
      try {
        for (;;) {
          const result = await setSignalAndReplot(reactive.view_id, name, value);
          if (result.plot_data && containerRef.current) {
            try {
              const next = JSON.parse(result.plot_data) as PlotlySpec;
              const layoutPatch: Partial<Plotly.Layout> = next.layout ?? {};
              await Plotly.react(
                containerRef.current,
                next.data,
                {
                  ...layoutPatch,
                  paper_bgcolor: "transparent",
                  plot_bgcolor: "transparent",
                },
              );
            } catch {
              console.warn("[PlotlyChart] Failed to parse replot response");
            }
          } else if (result.is_error) {
            console.warn("[PlotlyChart] Replot error:", result.error);
          }
          const next = pendingRef.current;
          pendingRef.current = null;
          if (!next) break;
          name = next.name;
          value = next.value;
        }
      } finally {
        inflightRef.current = false;
      }
    },
    [reactive.view_id, containerRef],
  );

  const onChange = useCallback(
    (sig: ReactiveSignalMeta, value: number) => {
      if (!Number.isFinite(value)) return;
      setValues((prev) => ({ ...prev, [sig.name]: value }));
      dispatch(sig.name, value);
    },
    [dispatch],
  );

  return (
    <div className="reactive-controls">
      {reactive.signals.map((sig) => (
        <SignalControl
          key={sig.name}
          sig={sig}
          value={values[sig.name] ?? sig.value}
          onChange={onChange}
        />
      ))}
    </div>
  );
}

interface SignalControlProps {
  sig: ReactiveSignalMeta;
  value: number;
  onChange: (sig: ReactiveSignalMeta, value: number) => void;
}

function SignalControl({ sig, value, onChange }: SignalControlProps) {
  const label = <span className="reactive-control-name">{sig.name}</span>;

  switch (sig.kind) {
    case "checkbox": {
      const checked = value >= 0.5;
      return (
        <div className="reactive-control reactive-control-checkbox">
          <label>
            <input
              type="checkbox"
              checked={checked}
              onChange={(e) => onChange(sig, e.target.checked ? 1 : 0)}
            />
            {label}
          </label>
        </div>
      );
    }
    case "dropdown": {
      const choices = sig.choices ?? [];
      return (
        <div className="reactive-control reactive-control-dropdown">
          <label>
            {label}
            <select
              value={String(value)}
              onChange={(e) => onChange(sig, Number(e.target.value))}
            >
              {choices.map((c) => (
                <option key={c} value={String(c)}>
                  {formatValue(c)}
                </option>
              ))}
            </select>
          </label>
        </div>
      );
    }
    case "number": {
      return (
        <div className="reactive-control reactive-control-number">
          <label>
            {label}
            <input
              type="number"
              min={sig.lo}
              max={sig.hi}
              step={(sig.hi - sig.lo) / 200 || 0.01}
              value={value}
              onChange={(e) => onChange(sig, Number(e.target.value))}
            />
          </label>
        </div>
      );
    }
    default: {
      // slider (and any unknown kind falls back to a range track)
      const step = (sig.hi - sig.lo) / 200 || 0.01;
      return (
        <div className="reactive-control">
          <label>
            {label}
            <input
              type="range"
              min={sig.lo}
              max={sig.hi}
              step={step}
              value={value}
              onChange={(e) => onChange(sig, Number(e.target.value))}
            />
            <span className="reactive-control-value">{formatValue(value)}</span>
          </label>
        </div>
      );
    }
  }
}

function formatValue(v: number): string {
  if (Math.abs(v) >= 100 || (v !== 0 && Math.abs(v) < 0.01)) {
    return v.toExponential(2);
  }
  return v.toFixed(3);
}
