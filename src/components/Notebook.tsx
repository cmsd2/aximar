import { useActiveTab } from "../store/notebookStore";
import { Cell } from "./Cell";
import { MarkdownCell } from "./MarkdownCell";
import { nbAddCell } from "../lib/notebook-commands";

function InsertBar({ afterId, beforeId }: { afterId?: string; beforeId?: string }) {
  const handleAddCode = async () => {
    await nbAddCell("code", undefined, afterId, beforeId);
  };
  const handleAddMarkdown = async () => {
    await nbAddCell("markdown", undefined, afterId, beforeId);
  };

  return (
    <div className="insert-bar">
      <div className="insert-bar-line" />
      <div className="insert-bar-buttons">
        <button className="insert-bar-btn" onClick={handleAddCode}>
          + Code
        </button>
        <button className="insert-bar-btn" onClick={handleAddMarkdown}>
          + Markdown
        </button>
      </div>
      <div className="insert-bar-line" />
    </div>
  );
}

interface NotebookProps {
  onViewDocs?: (name: string) => void;
}

export function Notebook({ onViewDocs }: NotebookProps) {
  const cells = useActiveTab().cells;

  return (
    <div className="notebook">
      <InsertBar beforeId={cells[0]?.id} />
      {cells.map((cell) => (
        <div key={cell.id}>
          {cell.cellType === "markdown" ? (
            <MarkdownCell cell={cell} />
          ) : (
            <Cell cell={cell} onViewDocs={onViewDocs} />
          )}
          <InsertBar afterId={cell.id} />
        </div>
      ))}
    </div>
  );
}
