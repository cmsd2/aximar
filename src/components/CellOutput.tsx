import type { CellOutput as CellOutputType } from "../types/notebook";
import { KatexOutput } from "./KatexOutput";
import { EnhancedErrorOutput } from "./EnhancedErrorOutput";

interface CellOutputProps {
  output: CellOutputType;
  cellId: string;
}

export function CellOutput({ output, cellId }: CellOutputProps) {
  if (output.isError && output.error) {
    return (
      <EnhancedErrorOutput
        error={output.error}
        errorInfo={output.errorInfo}
        cellId={cellId}
      />
    );
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
