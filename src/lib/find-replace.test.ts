import { describe, expect, it } from "vitest";
import { findMatches, replaceAllMatches } from "./find-replace";

describe("findMatches", () => {
  it("finds nothing for an empty query", () => {
    expect(findMatches("abc", "", true)).toEqual([]);
  });

  it("does not overlap matches", () => {
    expect(findMatches("aaa", "aa", true)).toEqual([{ start: 0, end: 2 }]);
    expect(findMatches("aaaa", "aa", true)).toEqual([
      { start: 0, end: 2 },
      { start: 2, end: 4 },
    ]);
  });

  it("respects case sensitivity", () => {
    expect(findMatches("Sin(x) + sin(y)", "sin", true)).toEqual([{ start: 9, end: 12 }]);
    expect(findMatches("Sin(x) + sin(y)", "sin", false)).toEqual([
      { start: 0, end: 3 },
      { start: 9, end: 12 },
    ]);
  });

  it("treats regex characters literally", () => {
    expect(findMatches("x^2 + (a.b)*", "(a.b)*", true)).toEqual([{ start: 6, end: 12 }]);
    expect(findMatches("a$b", "$", true)).toEqual([{ start: 1, end: 2 }]);
  });

  it("keeps offsets into the original text when lower-casing changes length", () => {
    // "İ".toLowerCase() is two code units long, which shifted later offsets
    const text = "İ x";
    expect(findMatches(text, "x", false)).toEqual([{ start: 2, end: 3 }]);
  });
});

describe("replaceAllMatches", () => {
  it("replaces non-overlapping matches", () => {
    expect(replaceAllMatches("aaa", "aa", "b", true)).toBe("ba");
    expect(replaceAllMatches("aaaa", "aa", "b", true)).toBe("bb");
  });

  it("replaces case-insensitively", () => {
    expect(replaceAllMatches("Sin(x) + sin(y)", "SIN", "cos", false)).toBe("cos(x) + cos(y)");
  });

  it("inserts the replacement literally", () => {
    expect(replaceAllMatches("x + y", "y", "$&$1", true)).toBe("x + $&$1");
  });

  it("leaves text without matches unchanged", () => {
    expect(replaceAllMatches("x + y", "z", "w", true)).toBe("x + y");
  });
});
