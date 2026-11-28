import { describe, expect, it } from "vitest";
import { changeBetween } from "./text-change";

/** Apply a change, to check it turns one text into the other */
function apply(text: string, from: number, to: number, insert: string): string {
  return text.slice(0, from) + insert + text.slice(to);
}

describe("changeBetween", () => {
  it("finds nothing to change in equal texts", () => {
    expect(changeBetween("x^2", "x^2")).toBeNull();
  });

  it("removes a character typed in the middle, as undoing it does", () => {
    // "ab|c", type x, undo: the cursor goes back between b and c
    const change = changeBetween("abxc", "abc")!;
    expect(change).toEqual({ from: 2, to: 3, insert: "" });
    expect(change.from + change.insert.length).toBe(2);
  });

  it("restores a deleted character with the cursor after it", () => {
    const change = changeBetween("ab", "abc")!;
    expect(change).toEqual({ from: 2, to: 2, insert: "c" });
    expect(change.from + change.insert.length).toBe(3);
  });

  it("removes a line copied above, leaving the lines after it alone", () => {
    // Alt+Shift+Up on the second line, then undo
    const before = "a: 1$\nb: 2$\nb: 2$\nc: 3$";
    const after = "a: 1$\nb: 2$\nc: 3$";
    const change = changeBetween(before, after)!;
    expect(apply(before, change.from, change.to, change.insert)).toBe(after);
    expect(change.to - change.from).toBe("b: 2$\n".length);
  });

  it("handles a repeated character", () => {
    const change = changeBetween("aa", "a")!;
    expect(apply("aa", change.from, change.to, change.insert)).toBe("a");
  });

  it("never splits a surrogate pair", () => {
    // 𝛼 and 𝛽 share their high surrogate
    const change = changeBetween("x𝛼y", "x𝛽y")!;
    expect(change).toEqual({ from: 1, to: 3, insert: "𝛽" });
  });
});
