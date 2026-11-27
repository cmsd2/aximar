import { describe, expect, it } from "vitest";
import type { Dependency } from "rollup-plugin-license";
import { noticeOf, noticesFile, thirdPartyNotices } from "./npm-notices";

function dep(fields: Partial<Dependency>): Dependency {
  return {
    name: "pkg",
    version: "1.0.0",
    maintainers: [],
    description: null,
    repository: null,
    homepage: null,
    private: false,
    license: "MIT",
    licenseText: "MIT text",
    noticeText: null,
    author: null,
    contributors: [],
    text: () => "",
    ...fields,
  } as Dependency;
}

describe("the npm notices", () => {
  const allow = thirdPartyNotices("x").allow;

  it("accept permissive licences and refuse others", () => {
    expect(allow.test(dep({ license: "MIT OR Apache-2.0" }))).toBe(true);
    expect(allow.test(dep({ license: "GPL-3.0-only" }))).toBe(false);
    expect(allow.test(dep({ license: null }))).toBe(false);
    expect(allow.failOnViolation && allow.failOnUnlicensed).toBe(true);
  });

  it("name the licence a package is used under", () => {
    const text = noticeOf(dep({ name: "both", license: "Apache-2.0 OR MIT", repository: { type: "git", url: "https://example.com/both" } }));
    expect(text).toContain("both@1.0.0\nLicence: MIT (offered: Apache-2.0 OR MIT)\nSource: https://example.com/both");
    expect(text).toContain("MIT text");
  });

  it("give an MIT package without a licence file the standard text", () => {
    const text = noticeOf(dep({ licenseText: null, author: { name: "Ada", email: null, url: null, text: () => "" } as never }));
    expect(text).toContain("Copyright (c) Ada");
    expect(text).toContain("Permission is hereby granted, free of charge");
  });

  it("list packages in name order under a header", () => {
    const file = noticesFile([dep({ name: "zeta" }), dep({ name: "alpha" })]);
    expect(file).toMatch(/^Aximar: notices/);
    expect(file.indexOf("alpha@")).toBeLessThan(file.indexOf("zeta@"));
    expect(file).toContain("the 2 packages below");
  });
});
