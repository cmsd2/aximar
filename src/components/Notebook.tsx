import { useNotebookStore } from "../store/notebookStore";
import { Cell } from "./Cell";
import { MarkdownCell } from "./MarkdownCell";

export function Notebook() {
  const cells = useNotebookStore((s) => s.cells);

  return (
    <div className="notebook">
      {cells.map((cell) =>
        cell.cellType === "markdown" ? (
          <MarkdownCell key={cell.id} cell={cell} />
        ) : (
          <Cell key={cell.id} cell={cell} />
        )
      )}
    </div>
  );
}
