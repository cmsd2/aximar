import { describe, expect, it } from "vitest";
import { nextSelection, selectHowOf } from "./selection-utils";
import type { Cell } from "../types/notebook";

const cells = ["a", "b", "c", "d"].map((id) => ({ id, cellType: "code", input: "", output: null, status: "idle" }) as Cell);

describe("nextSelection", () => {
  it("selects only the clicked cell on a plain click", () => {
    expect(nextSelection(cells, ["a", "b"], "c", "only")).toEqual(["c"]);
  });

  it("clears the selection on a plain click on the only selected cell", () => {
    expect(nextSelection(cells, ["b"], "b", "only")).toEqual([]);
  });

  it("keeps one cell of several on a plain click on it", () => {
    expect(nextSelection(cells, ["a", "b"], "b", "only")).toEqual(["b"]);
  });

  it("adds and removes one cell with Cmd or Ctrl", () => {
    expect(nextSelection(cells, ["a"], "c", "toggle")).toEqual(["a", "c"]);
    expect(nextSelection(cells, ["a", "c"], "a", "toggle")).toEqual(["c"]);
  });

  it("selects from the last selected cell with Shift, either way", () => {
    expect(nextSelection(cells, ["b"], "d", "range")).toEqual(["b", "c", "d"]);
    expect(nextSelection(cells, ["d"], "a", "range")).toEqual(["a", "b", "c", "d"]);
  });

  it("selects one cell with Shift when nothing is selected", () => {
    expect(nextSelection(cells, [], "c", "range")).toEqual(["c"]);
  });
});

describe("selectHowOf", () => {
  it("reads Shift before Cmd or Ctrl", () => {
    expect(selectHowOf({ shiftKey: true, metaKey: true, ctrlKey: false })).toBe("range");
    expect(selectHowOf({ shiftKey: false, metaKey: true, ctrlKey: false })).toBe("toggle");
    expect(selectHowOf({ shiftKey: false, metaKey: false, ctrlKey: true })).toBe("toggle");
    expect(selectHowOf({ shiftKey: false, metaKey: false, ctrlKey: false })).toBe("only");
  });
});
