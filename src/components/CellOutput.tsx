import type { CellOutput as CellOutputType } from "../types/notebook";
import { KatexOutput } from "./KatexOutput";
import { ErrorOutput } from "./ErrorOutput";

interface CellOutputProps {
  output: CellOutputType;
}

export function CellOutput({ output }: CellOutputProps) {
  if (output.isError && output.error) {
    return <ErrorOutput error={output.error} />;
  }

  const hasLatex = output.latex !== null && output.latex !== "";
  const hasText = output.textOutput !== "";

  return (
    <div className="cell-output">
      {hasLatex && <KatexOutput latex={output.latex!} />}
      {hasText && !hasLatex && (
        <pre className="text-output">{output.textOutput}</pre>
      )}
      {!hasLatex && !hasText && (
        <span className="text-output empty-output">(no output)</span>
      )}
    </div>
  );
}
