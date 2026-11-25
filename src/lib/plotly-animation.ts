/**
 * Animated Plotly figures (ax-plots `animate(...)`): what the renderer
 * adjusts so they play smoothly.
 *
 * - Steady axes. With autorange, an axis fits the frame on show: as an
 *   animation plays, the y-axis rescales every frame and the curve
 *   jitters against it, and a figure played without a full redraw keeps
 *   the first frame's range and cuts off later frames. So each
 *   autoranged axis gets one range that covers every frame.
 * - Every point of a line. Plotly drops nearly collinear points from a
 *   line by default, so each frame's line has a different number of
 *   points, and the tween between frames pairs them wrongly: the line
 *   tears, mostly towards its end. ax-plots after 0.2.1 turns this off
 *   itself; the renderer does too, for figures from older versions.
 *
 * The VS Code extension's renderer has the same code
 * (cmsd2/maxima-notebook, src/renderers/maxima/animation.ts).
 */

interface Trace {
  type?: string;
  mode?: string;
  line?: Record<string, unknown>;
  x?: unknown;
  y?: unknown;
  z?: unknown;
  xaxis?: string;
  yaxis?: string;
  scene?: string;
}

interface Frame {
  data?: Trace[];
  traces?: number[];
}

export interface Figure {
  data: unknown[];
  layout?: Record<string, unknown>;
  frames?: unknown[];
}

/** Trace types plotted on x/y axes, and on a 3D scene's axes. */
const CARTESIAN = new Set(["scatter", "scattergl"]);
const SCENE = new Set(["scatter3d", "surface", "mesh3d"]);
/** Axis types whose values aren't plain numbers. */
const NOT_LINEAR = new Set(["log", "date", "category", "multicategory"]);

function collect(values: unknown, out: number[]): void {
  if (Array.isArray(values)) {
    for (const v of values) {
      collect(v, out);
    }
  } else if (typeof values === "number" && Number.isFinite(values)) {
    out.push(values);
  }
}

/** Where an axis lives in the layout: a key path such as ["yaxis"] or ["scene", "zaxis"]. */
type AxisPath = [string] | [string, string];

function axisPaths(trace: Trace, type: string | undefined): [AxisPath, keyof Trace][] {
  const t = type ?? "scatter";
  if (CARTESIAN.has(t)) {
    const x = (trace.xaxis ?? "x").replace(/^x/, "xaxis");
    const y = (trace.yaxis ?? "y").replace(/^y/, "yaxis");
    return [
      [[x], "x"],
      [[y], "y"],
    ];
  }
  if (SCENE.has(t)) {
    const scene = trace.scene ?? "scene";
    return [
      [[scene, "xaxis"], "x"],
      [[scene, "yaxis"], "y"],
      [[scene, "zaxis"], "z"],
    ];
  }
  return [];
}

/**
 * The layout with fixed ranges for the figure's autoranged axes, or the
 * layout unchanged when the figure has no frames. An axis keeps a range
 * or autorange setting of its own.
 */
export function fixAxesForFrames(figure: Figure): Record<string, unknown> | undefined {
  const frames = (figure.frames ?? []) as Frame[];
  if (frames.length === 0) {
    return figure.layout;
  }
  const data = figure.data as Trace[];
  const values = new Map<string, { path: AxisPath; numbers: number[] }>();
  const add = (trace: Trace, type: string | undefined) => {
    for (const [path, field] of axisPaths(trace, type)) {
      const key = path.join(".");
      const entry = values.get(key) ?? { path, numbers: [] };
      collect(trace[field], entry.numbers);
      values.set(key, entry);
    }
  };
  for (const trace of data) {
    add(trace, trace.type);
  }
  for (const frame of frames) {
    (frame.data ?? []).forEach((trace, i) => {
      // A frame's trace updates data[traces[i]] (or data[i]), whose type it shares
      const target = data[frame.traces?.[i] ?? i];
      add(trace, trace.type ?? target?.type);
    });
  }

  const layout: Record<string, unknown> = { ...(figure.layout ?? {}) };
  for (const { path, numbers } of values.values()) {
    if (numbers.length === 0) {
      continue;
    }
    const parent =
      path.length === 2
        ? ((layout[path[0]] = { ...((layout[path[0]] as Record<string, unknown>) ?? {}) }) as Record<
            string,
            unknown
          >)
        : layout;
    const name = path[path.length - 1];
    const axis = (parent[name] as Record<string, unknown> | undefined) ?? {};
    if (axis.range !== undefined || axis.autorange === false || NOT_LINEAR.has(axis.type as string)) {
      continue;
    }
    let lo = Infinity;
    let hi = -Infinity;
    for (const n of numbers) {
      lo = Math.min(lo, n);
      hi = Math.max(hi, n);
    }
    // Room around the values, as autorange leaves; the x-axis of a line
    // plot runs edge to edge
    const span = hi - lo;
    const pad = name === "xaxis" && path.length === 1 ? 0 : span > 0 ? span * 0.05 : Math.abs(hi) * 0.05 || 0.5;
    parent[name] = { ...axis, range: [lo - pad, hi + pad], autorange: false };
  }
  return layout;
}

/**
 * The figure's traces and frames with line simplification off on every
 * line trace, or the figure unchanged when it has no frames.
 */
export function keepEveryLinePoint(figure: Figure): Figure {
  const frames = (figure.frames ?? []) as Frame[];
  if (frames.length === 0) {
    return figure;
  }
  const data = figure.data as Trace[];
  const off = (trace: Trace, type: string | undefined): Trace =>
    CARTESIAN.has(type ?? "scatter") ? { ...trace, line: { ...(trace.line ?? {}), simplify: false } } : trace;
  return {
    ...figure,
    data: data.map((t) => off(t, t.type)),
    frames: frames.map((frame) => ({
      ...frame,
      data: (frame.data ?? []).map((t, i) => off(t, t.type ?? data[frame.traces?.[i] ?? i]?.type)),
    })),
  };
}
