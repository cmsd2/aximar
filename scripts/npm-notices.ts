// The notices file for the npm packages in the window's bundle, written
// by rollup-plugin-license during `vite build`.  See docs/licences.md.
import type { Dependency } from "rollup-plugin-license";
import { chosenLicence, isAccepted } from "./accepted-licences";

function repositoryOf(dependency: Dependency): string | null {
  const repo = dependency.repository;
  const url = typeof repo === "string" ? repo : (repo?.url ?? null);
  return url ?? dependency.homepage ?? null;
}

function authorOf(dependency: Dependency): string | null {
  const a = dependency.author;
  return a ? [a.name, a.email ? `<${a.email}>` : null].filter(Boolean).join(" ") : null;
}

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

/**
 * For a package that ships no licence file: its copyright line and, for
 * MIT, the licence's standard text, which MIT asks every copy to carry
 */
function standardText(licence: string | null, author: string | null): string {
  const copyright = `Copyright (c) ${author ?? "its authors"}`;
  if (licence === "MIT") return `The package ships no licence file; its package.json names MIT.\n\n${copyright}\n\n${MIT_TEXT}`;
  return `The package ships no licence file; its package.json names ${licence}.\n\n${copyright}`;
}

/** One package's entry: name, licence, source, then its licence text */
export function noticeOf(dependency: Dependency): string {
  const chosen = chosenLicence(dependency.license);
  const offered = dependency.license && chosen !== dependency.license ? ` (offered: ${dependency.license})` : "";
  const lines = [`${dependency.name}@${dependency.version}`, `Licence: ${chosen ?? dependency.license}${offered}`];
  const source = repositoryOf(dependency);
  if (source) lines.push(`Source: ${source}`);
  const text = [dependency.licenseText, dependency.noticeText].filter(Boolean).join("\n\n").trim();
  lines.push("", text || standardText(chosen, authorOf(dependency)));
  return lines.join("\n");
}

export function noticesFile(dependencies: Dependency[]): string {
  const sorted = [...dependencies].sort((a, b) => `${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`));
  const header = [
    "Aximar: notices for the open-source npm packages in the app's window",
    "",
    `Aximar's window includes code from the ${sorted.length} packages below.  Each`,
    "is listed with the licence Aximar uses it under, its source, and its",
    "copyright notice and licence text.  The Rust crates' notices are in",
    "third-party-rust.html, and the fonts' in this folder's OFL files.",
  ].join("\n");
  const rule = "\n\n" + "-".repeat(72) + "\n\n";
  return header + rule + sorted.map(noticeOf).join(rule) + "\n";
}

/** rollup-plugin-license's thirdParty options: the notices file, and the accepted-licence check */
export function thirdPartyNotices(file: string) {
  return {
    includePrivate: false,
    output: { file, template: noticesFile },
    allow: {
      test: (dependency: Dependency) => isAccepted(dependency.license),
      failOnUnlicensed: true,
      failOnViolation: true,
    },
  };
}
