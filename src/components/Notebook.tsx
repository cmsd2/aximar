import { useNotebookStore } from "../store/notebookStore";
import { Cell } from "./Cell";
import { MarkdownCell } from "./MarkdownCell";

function InsertBar({ afterId }: { afterId?: string }) {
  const addCell = useNotebookStore((s) => s.addCell);
  const addMarkdownCell = useNotebookStore((s) => s.addMarkdownCell);

  return (
    <div className="insert-bar">
      <div className="insert-bar-line" />
      <div className="insert-bar-buttons">
        <button className="insert-bar-btn" onClick={() => addCell(afterId)}>
          + Code
        </button>
        <button className="insert-bar-btn" onClick={() => addMarkdownCell(afterId)}>
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
  const cells = useNotebookStore((s) => s.cells);

  return (
    <div className="notebook">
      <InsertBar />
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
