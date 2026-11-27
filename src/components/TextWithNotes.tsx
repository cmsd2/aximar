import { useState } from "react";
import { useLingui } from "@lingui/react/macro";
import type { Note } from "../types/maxima";
import { messageText } from "../lib/messages";
import { RichTextOutput } from "./RichTextOutput";

interface TextWithNotesProps {
  /** The cell's text output */
  text: string;
  notes: Note[];
}

/**
 * A cell's text output, with each note in place of the Maxima lines it
 * covers.  Text outside any note shows as Maxima printed it.
 */
export function TextWithNotes({ text, notes }: TextWithNotesProps) {
  if (notes.length === 0) return <RichTextOutput text={text} />;

  const lines = text.split("\n");
  const parts: React.ReactNode[] = [];
  let line = 0;
  for (const note of [...notes].sort((a, b) => a.start - b.start)) {
    if (note.start < line || note.end > lines.length) continue;
    if (note.start > line) {
      parts.push(<RichTextOutput key={`text-${line}`} text={lines.slice(line, note.start).join("\n")} />);
    }
    parts.push(<NoteLine key={`note-${note.start}`} note={note} lines={lines.slice(note.start, note.end)} />);
    line = note.end;
  }
  if (line < lines.length) {
    const rest = lines.slice(line).join("\n");
    if (rest !== "") parts.push(<RichTextOutput key={`text-${line}`} text={rest} />);
  }
  return <>{parts}</>;
}

/** One note: what it means, and Maxima's own lines on request */
export function NoteLine({ note, lines }: { note: Note; lines: string[] }) {
  const { t, i18n } = useLingui();
  const [showMaxima, setShowMaxima] = useState(false);
  return (
    <div className="output-note">
      <div className="output-note-line">
        <span className="output-note-mark" aria-hidden="true">
          ⓘ
        </span>
        <span className="output-note-text">{messageText(i18n, note.message)}</span>
        <button className="output-note-toggle" onClick={() => setShowMaxima(!showMaxima)}>
          {showMaxima
            ? t({
                message: "Hide Maxima's text",
                comment: "Button on a note in a cell's output: hides the lines Maxima printed, which the note explains",
              })
            : t({
                message: "Show Maxima's text",
                comment: "Button on a note in a cell's output: shows the lines Maxima printed, which the note explains",
              })}
        </button>
      </div>
      {showMaxima && <pre className="output-note-maxima">{lines.join("\n")}</pre>}
    </div>
  );
}
