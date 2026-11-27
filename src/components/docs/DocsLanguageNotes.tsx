import { useState } from "react";
import { useLingui } from "@lingui/react/macro";
import type { I18n } from "@lingui/core";
import { Button } from "../ui/Button";
import { StatusBox } from "../ui/StatusBox";
import type { DocsLanguages, MaximaFunction } from "../../types/catalog";

/** A language's name in the window's language, such as "inglés" in Spanish */
export function nameIn(i18n: I18n, tag: string): string {
  try {
    return new Intl.DisplayNames([i18n.locale], { type: "language" }).of(tag) ?? tag;
  } catch {
    return tag;
  }
}

/**
 * Above a function's documentation: a translated summary and what language
 * the rest is in, a note when the function has nothing in the help's
 * language or its translation may be out of date, and how to install full
 * documentation in the help's language when it isn't
 */
export function DocsLanguageNotes({ fn, info }: { fn: MaximaFunction; info: DocsLanguages | null }) {
  const { t, i18n } = useLingui();
  const langs = fn.languages;
  const help = info?.effective ?? "en";
  const helpEntry = info?.languages.find((l) => l.tag === help);
  const notInstalled = help !== "en" && helpEntry && !helpEntry.full;
  const helpName = nameIn(i18n, help);
  const english = nameIn(i18n, "en");

  return (
    <div className="docs-language-notes">
      {notInstalled && <MissingPack language={helpName} pack={helpEntry.pack} />}

      {langs?.fallback_from && (
        <StatusBox tone="info" status="docs-fallback">
          {t({
            message: `This function isn't documented in ${helpName}. Its documentation is in ${english}.`,
            comment: "Docs panel: note above a function's documentation when the help's language has nothing for it. helpName and english are language names, such as Spanish and English",
          })}
        </StatusBox>
      )}

      {langs?.summary_text && (
        <>
          <p className="docs-translated-summary" lang={langs.summary}>
            {langs.summary_text}
          </p>
          <StatusBox tone="info" status="docs-body-language">
            {(() => {
              const bodyName = nameIn(i18n, langs.body);
              return t({
                message: `The full documentation below is in ${bodyName}.`,
                comment: "Docs panel: note between a translated summary and the full documentation, which is in another language. bodyName is a language name, such as English",
              });
            })()}
          </StatusBox>
        </>
      )}

      {langs?.stale && (
        <StatusBox
          tone="warn"
          status="docs-stale"
          detail={
            <span className="docs-english-signatures">
              {(langs.english_signatures ?? []).map((sig) => (
                <code key={sig}>{sig}</code>
              ))}
            </span>
          }
        >
          {t({
            message: `This translation may be out of date. The ${english} documentation lists these signatures:`,
            comment: "Docs panel: note above a translation whose signatures differ from the English documentation's; the English signatures follow. english is a language name",
          })}
        </StatusBox>
      )}
    </div>
  );
}

/** Full documentation in the help's language isn't installed: how to get it */
function MissingPack({ language, pack }: { language: string; pack: string | null }) {
  const { t } = useLingui();
  const [copied, setCopied] = useState(false);
  const command = pack ? `mxpm install ${pack}` : null;
  return (
    <StatusBox
      tone="info"
      status="docs-missing-pack"
      detail={
        command && (
          <span className="docs-install-command">
            <code>{command}</code>
            <Button
              size="sm"
              variant="quiet"
              onClick={() =>
                navigator.clipboard.writeText(command).then(() => {
                  setCopied(true);
                  setTimeout(() => setCopied(false), 2000);
                })
              }
            >
              {copied
                ? t({ message: "Copied", comment: "Docs panel: the install command's copy button, for two seconds after a copy" })
                : t({ message: "Copy", comment: "Docs panel button: copies the mxpm install command to the clipboard" })}
            </Button>
          </span>
        )
      }
    >
      {command
        ? t({
            message: `Full documentation in ${language} isn't installed. To install it, run:`,
            comment: "Docs panel: note when the help's language has summaries but no full documentation installed; an mxpm command follows. language is a language name",
          })
        : t({
            message: `Full documentation in ${language} isn't available yet.`,
            comment: "Docs panel: note when there is no package of full documentation in the help's language. language is a language name",
          })}
    </StatusBox>
  );
}
