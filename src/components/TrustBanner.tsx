import { useActiveTab, useNotebookStore } from "../store/notebookStore";
import { nbTrustNotebook } from "../lib/notebook-commands";

/**
 * Notebook-level trust banner. Shown when an untrusted notebook tries to
 * execute a cell containing dangerous functions (system, batch, etc.).
 */
export function TrustBanner() {
  const tab = useActiveTab();
  const setPendingTrustFunctions = useNotebookStore((s) => s.setPendingTrustFunctions);

  if (tab.trusted || !tab.pendingTrustFunctions) return null;

  const handleTrust = async () => {
    await nbTrustNotebook(true);
    setPendingTrustFunctions(null);
  };

  const handleDismiss = () => {
    setPendingTrustFunctions(null);
  };

  return (
    <div className="trust-banner">
      <span className="trust-banner-warning">
        &#9888; This notebook uses dangerous functions:{" "}
        <strong>{tab.pendingTrustFunctions.join(", ")}</strong>.
        Trust it to allow execution.
      </span>
      <button className="trust-banner-btn trust-btn" onClick={handleTrust}>
        Trust Notebook
      </button>
      <button className="trust-banner-btn dismiss-btn" onClick={handleDismiss}>
        Dismiss
      </button>
    </div>
  );
}
