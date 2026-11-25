import { describe, expect, it } from "vitest";
import { fixAxesForFrames, keepEveryLinePoint } from "./plotly-animation";

const xs = [0, 1, 2];

describe("fixAxesForFrames", () => {
  it("leaves a figure without frames alone", () => {
    const layout = { title: "t" };
    expect(fixAxesForFrames({ data: [{ x: xs, y: [0, 1, 2] }], layout })).toBe(layout);
  });

  it("gives the y-axis one range for every frame, with room around it", () => {
    const layout = fixAxesForFrames({
      data: [{ type: "scatter", x: xs, y: [0, 0.3, 0] }],
      frames: [
        { name: "0", data: [{ y: [0, 0.3, 0] }], traces: [0] },
        { name: "1", data: [{ y: [-1, 1, 0] }], traces: [0] },
      ],
    });
    expect(layout?.yaxis).toEqual({ range: [-1.1, 1.1], autorange: false });
    // A line plot's x-axis runs edge to edge
    expect(layout?.xaxis).toEqual({ range: [0, 2], autorange: false });
  });

  it("includes static traces, and frame traces take their type from the trace they update", () => {
    const layout = fixAxesForFrames({
      data: [
        { type: "scatter", x: xs, y: [-3, 0, 3] },
        { type: "scatter", x: xs, y: [0, 0, 0] },
      ],
      frames: [{ data: [{ y: [0, 5, 0] }], traces: [1] }],
    });
    const [lo, hi] = (layout?.yaxis as { range: number[] }).range;
    expect(lo).toBeCloseTo(-3.4);
    expect(hi).toBeCloseTo(5.4);
  });

  it("keeps a range or autorange the figure sets itself", () => {
    const layout = fixAxesForFrames({
      data: [{ x: xs, y: [0, 1, 2] }],
      layout: { yaxis: { range: [-5, 5] }, xaxis: { autorange: false } },
      frames: [{ data: [{ y: [9, 9, 9] }] }],
    });
    expect(layout?.yaxis).toEqual({ range: [-5, 5] });
    expect(layout?.xaxis).toEqual({ autorange: false });
  });

  it("fixes a 3D scene's axes, including a surface's grid of z values", () => {
    const layout = fixAxesForFrames({
      data: [{ type: "surface", x: xs, y: xs, z: [[0, 1], [1, 0]] }],
      frames: [{ data: [{ z: [[0, 2], [2, 0]] }], traces: [0] }],
    });
    const scene = layout?.scene as Record<string, { range: number[] }>;
    expect(scene.zaxis.range).toEqual([-0.1, 2.1]);
    expect(scene.xaxis.range).toEqual([-0.1, 2.1]);
  });

  it("skips log axes, and trace types it doesn't know", () => {
    const layout = fixAxesForFrames({
      data: [{ type: "heatmap", z: [[1, 2]] }, { x: xs, y: [1, 10, 100] }],
      layout: { yaxis: { type: "log" } },
      frames: [{ data: [{ z: [[5, 6]] }], traces: [0] }],
    });
    expect(layout?.yaxis).toEqual({ type: "log" });
    expect(layout?.scene).toBeUndefined();
  });
});

describe("keepEveryLinePoint", () => {
  it("turns off line simplification on line traces and their frames", () => {
    const fig = keepEveryLinePoint({
      data: [
        { type: "scatter", x: xs, y: xs, line: { color: "red" } },
        { type: "heatmap", z: [[1]] },
      ],
      frames: [{ data: [{ y: xs }], traces: [0] }],
    });
    expect((fig.data[0] as { line: unknown }).line).toEqual({ color: "red", simplify: false });
    expect((fig.data[1] as { line?: unknown }).line).toBeUndefined();
    const frame = (fig.frames as { data: { line: unknown }[] }[])[0];
    expect(frame.data[0].line).toEqual({ simplify: false });
  });

  it("leaves a figure without frames alone", () => {
    const fig = { data: [{ type: "scatter", x: xs, y: xs }] };
    expect(keepEveryLinePoint(fig)).toBe(fig);
  });
});
