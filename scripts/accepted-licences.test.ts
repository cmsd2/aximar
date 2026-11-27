import { describe, expect, it } from "vitest";
import { chosenLicence, isAccepted } from "./accepted-licences";

describe("the accepted licences", () => {
  it("npm_licences_are_accepted", () => {
    expect(isAccepted("MIT")).toBe(true);
    expect(isAccepted("MIT OR Apache-2.0")).toBe(true);
    expect(isAccepted("(MIT AND Zlib)")).toBe(true);
    expect(isAccepted("GPL-3.0-only")).toBe(false);
    expect(isAccepted(null)).toBe(false);
    expect(isAccepted("")).toBe(false);
  });

  it("choose the first accepted licence offered", () => {
    expect(chosenLicence("MIT OR Apache-2.0 OR LGPL-2.1-or-later")).toBe("MIT");
    expect(chosenLicence("Apache-2.0 OR MIT")).toBe("MIT");
    expect(chosenLicence("LGPL-2.1-or-later OR MPL-2.0")).toBe("MPL-2.0");
    expect(chosenLicence("GPL-2.0-only OR GPL-3.0-only")).toBeNull();
  });

  it("need every licence an AND names", () => {
    expect(chosenLicence("MIT AND BSD-3-Clause")).toBe("MIT AND BSD-3-Clause");
    expect(chosenLicence("MIT AND GPL-3.0-only")).toBeNull();
    expect(chosenLicence("(MIT OR Apache-2.0) AND Unicode-3.0")).toBe("MIT AND Unicode-3.0");
  });

  it("read exceptions and the older slash form", () => {
    expect(chosenLicence("Apache-2.0 WITH LLVM-exception")).toBe("Apache-2.0 WITH LLVM-exception");
    expect(chosenLicence("GPL-2.0-only WITH Classpath-exception-2.0")).toBeNull();
    expect(chosenLicence("MIT/Apache-2.0")).toBe("MIT");
    expect(chosenLicence("Apache-2.0 / MIT")).toBe("MIT");
  });

  it("refuse what it can't read", () => {
    expect(chosenLicence("(MIT OR")).toBeNull();
    expect(chosenLicence("SEE LICENSE IN LICENSE.txt")).toBeNull();
  });
});

describe("about.toml", () => {
  it("accepts the same licences as the npm build, in the same order", async () => {
    const { readFileSync } = await import("node:fs");
    const toml = readFileSync(new URL("../about.toml", import.meta.url), "utf8");
    const block = toml.match(/^accepted = \[([\s\S]*?)\]/m)?.[1] ?? "";
    const rust = [...block.matchAll(/"([^"]+)"/g)].map((m) => m[1]);
    const { ACCEPTED_LICENCES } = await import("./accepted-licences");
    expect(rust).toEqual([...ACCEPTED_LICENCES]);
  });
});
