import { describe, expect, it } from "vitest";
import type { Dependency } from "rollup-plugin-license";
import { groupNotices, noticesPage, sourceUrl, thirdPartyNotices } from "./npm-notices";

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

  it("group packages that share a licence text, and keep different texts apart", () => {
    const groups = groupNotices([
      dep({ name: "b" }),
      dep({ name: "a" }),
      dep({ name: "c", licenseText: "Copyright (c) Someone else\nMIT text" }),
      dep({ name: "d", license: "ISC", licenseText: "ISC text" }),
    ]);
    expect(groups.map((g) => [g.licence, g.packages.map((p) => p.name)])).toEqual([
      ["ISC", ["d"]],
      ["MIT", ["a", "b"]],
      ["MIT", ["c"]],
    ]);
  });

  it("name the licence a package is used under", () => {
    const [group] = groupNotices([dep({ name: "both", license: "Apache-2.0 OR MIT" })]);
    expect(group.licence).toBe("MIT");
    expect(group.packages[0].offered).toBe("Apache-2.0 OR MIT");
  });

  it("give an MIT package without a licence file the standard text", () => {
    const [group] = groupNotices([dep({ licenseText: null, author: { name: "Ada", email: null, url: null, text: () => "" } as never })]);
    expect(group.text).toContain("Copyright (c) Ada");
    expect(group.text).toContain("Permission is hereby granted, free of charge");
  });

  it("turn package.json's repository into a web address", () => {
    expect(sourceUrl(dep({ repository: { type: "git", url: "git+https://github.com/codemirror/view.git" } }))).toBe(
      "https://github.com/codemirror/view",
    );
    expect(sourceUrl(dep({ repository: "git://github.com/a/b.git" }))).toBe("https://github.com/a/b");
    expect(sourceUrl(dep({ repository: "github:a/b" }))).toBe("https://github.com/a/b");
    expect(sourceUrl(dep({ repository: "a/b" }))).toBe("https://github.com/a/b");
    expect(sourceUrl(dep({ repository: "git@gitlab.com:a/b.git" }))).toBe("https://gitlab.com/a/b");
    expect(sourceUrl(dep({ homepage: "https://example.com" }))).toBe("https://example.com");
  });

  it("render a page with a marker, a contents list, and escaped texts", () => {
    const page = noticesPage([dep({ name: "x", licenseText: "a <b> & c", repository: "x/y" })]);
    expect(page).toContain('<meta name="aximar-notices" content="npm">');
    expect(page).toContain("<li>MIT: 1 package</li>");
    expect(page).toContain('x 1.0.0: <a href="https://github.com/x/y">source</a>');
    expect(page).toContain("a &lt;b&gt; &amp; c");
  });
});
