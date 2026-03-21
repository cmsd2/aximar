import { useNotebookStore } from "../store/notebookStore";
import { Cell } from "./Cell";

export function Notebook() {
  const cells = useNotebookStore((s) => s.cells);

  return (
    <div className="notebook">
      {cells.map((cell) => (
        <Cell key={cell.id} cell={cell} />
      ))}
    </div>
  );
}
