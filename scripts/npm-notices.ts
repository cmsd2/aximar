// The notices page for the npm packages in the window's bundle, written
// by rollup-plugin-license during `vite build`.  It has the same shape as
// the Rust crates' page: a contents list by licence, then each licence
// text once, with the packages Aximar uses under it and a link to each
// one's source.  See docs/licences.md.
import type { Dependency } from "rollup-plugin-license";
import { chosenLicence, isAccepted } from "./accepted-licences";

const MIT_TEXT = `Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.`;

function authorOf(dependency: Dependency): string | null {
  const a = dependency.author;
  return a ? [a.name, a.email ? `<${a.email}>` : null].filter(Boolean).join(" ") : null;
}

/**
 * For a package that ships no licence file: its copyright line and, for
 * MIT, the licence's standard text, which MIT asks every copy to carry
 */
function standardText(licence: string | null, author: string | null): string {
  const copyright = `Copyright (c) ${author ?? "its authors"}`;
  if (licence === "MIT") return `The package ships no licence file; its package.json names MIT.\n\n${copyright}\n\n${MIT_TEXT}`;
  return `The package ships no licence file; its package.json names ${licence}.\n\n${copyright}`;
}

/** A package's source as a web address, from package.json's repository or homepage */
export function sourceUrl(dependency: Dependency): string | null {
  const repo = dependency.repository;
  let url = typeof repo === "string" ? repo : (repo?.url ?? null);
  if (url) {
    const short = url.match(/^(github|gitlab|bitbucket):(.+)$/);
    if (short) url = `https://${short[1] === "bitbucket" ? "bitbucket.org" : `${short[1]}.com`}/${short[2]}`;
    else if (/^[\w.-]+\/[\w.-]+$/.test(url)) url = `https://github.com/${url}`;
    url = url
      .replace(/^git\+/, "")
      .replace(/^git:\/\//, "https://")
      .replace(/^ssh:\/\/git@/, "https://")
      .replace(/^git@([^:]+):/, "https://$1/")
      .replace(/\.git(#.*)?$/, "");
    if (/^https?:\/\//.test(url)) return url;
  }
  return dependency.homepage ?? null;
}

export interface Group {
  /** The licence, or licences joined by AND, the packages are used under */
  licence: string;
  text: string;
  packages: { name: string; version: string; offered: string | null; source: string | null }[];
}

/** The packages by licence text: each text once, with the packages that use it */
export function groupNotices(dependencies: Dependency[]): Group[] {
  const groups = new Map<string, Group>();
  for (const d of dependencies) {
    const licence = chosenLicence(d.license) ?? d.license ?? "unknown";
    const fileText = [d.licenseText, d.noticeText].filter(Boolean).join("\n\n").trim();
    const text = fileText || standardText(licence, authorOf(d));
    const key = `${licence}\n${text}`;
    const group = groups.get(key) ?? { licence, text, packages: [] };
    group.packages.push({
      name: d.name ?? "(unnamed)",
      version: d.version ?? "",
      offered: d.license && d.license !== licence ? d.license : null,
      source: sourceUrl(d),
    });
    groups.set(key, group);
  }
  const byName = (a: { name: string; version: string }, b: { name: string; version: string }) =>
    a.name.localeCompare(b.name) || a.version.localeCompare(b.version, undefined, { numeric: true });
  return [...groups.values()]
    .map((g) => ({ ...g, packages: g.packages.sort(byName) }))
    .sort((a, b) => a.licence.localeCompare(b.licence) || byName(a.packages[0], b.packages[0]));
}

const escape = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

export function noticesPage(dependencies: Dependency[]): string {
  const groups = groupNotices(dependencies);
  const total = groups.reduce((n, g) => n + g.packages.length, 0);
  const perLicence = new Map<string, number>();
  for (const g of groups) perLicence.set(g.licence, (perLicence.get(g.licence) ?? 0) + g.packages.length);
  const overview = [...perLicence]
    .map(([licence, n]) => `  <li>${escape(licence)}: ${n} ${n === 1 ? "package" : "packages"}</li>`)
    .join("\n");
  const sections = groups
    .map((g, i) => {
      const items = g.packages
        .map((p) => {
          const offered = p.offered ? ` (offered: ${escape(p.offered)})` : "";
          const source = p.source ? `: <a href="${escape(p.source)}">source</a>` : "";
          return `  <li>${escape(p.name)} ${escape(p.version)}${offered}${source}</li>`;
        })
        .join("\n");
      return `<section id="npm-licence-${i + 1}">\n<h2>${escape(g.licence)}</h2>\n<ul>\n${items}\n</ul>\n<pre>${escape(g.text)}</pre>\n</section>`;
    })
    .join("\n\n");
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="aximar-notices" content="npm">
<title>npm packages: licence notices</title>
<style>
  :root { color-scheme: light dark; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }
  body { max-width: 860px; margin: 24px auto; padding: 0 16px; line-height: 1.5; }
  pre { white-space: pre-wrap; font-size: 12px; }
  li { margin: 2px 0; }
</style>
</head>
<body>
<h1>npm packages: licence notices</h1>
<p>Aximar's window includes code from the ${total} npm packages below. Each licence text appears once, with the packages Aximar uses under it, and each package links its source.</p>
<h2>Licences</h2>
<ul>
${overview}
</ul>

${sections}
</body>
</html>
`;
}

/** rollup-plugin-license's thirdParty options: the notices page, and the accepted-licence check */
export function thirdPartyNotices(file: string) {
  return {
    includePrivate: false,
    output: { file, template: noticesPage },
    allow: {
      test: (dependency: Dependency) => isAccepted(dependency.license),
      failOnUnlicensed: true,
      failOnViolation: true,
    },
  };
}
