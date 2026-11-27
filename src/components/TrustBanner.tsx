import { Trans, useLingui } from "@lingui/react/macro";
import { useActiveTab, useNotebookStore } from "../store/notebookStore";
import { nbTrustNotebook } from "../lib/notebook-commands";

/**
 * Notebook-level trust banner. Shown when an untrusted notebook tries to
 * execute a cell containing dangerous functions (system, batch, etc.).
 */
export function TrustBanner() {
  const { t } = useLingui();
  const tab = useActiveTab();
  const setPendingTrustFunctions = useNotebookStore((s) => s.setPendingTrustFunctions);

  if (tab.trusted || !tab.pendingTrustFunctions) return null;

  const handleTrust = async () => {
    await nbTrustNotebook(true);
    setPendingTrustFunctions(tab.id, null);
  };

  const handleDismiss = () => {
    setPendingTrustFunctions(tab.id, null);
  };

  const functions = tab.pendingTrustFunctions.join(", ");

  return (
    <div className="trust-banner">
      <span className="trust-banner-warning">
        &#9888;{" "}
        <Trans comment="Trust banner above the notebook, shown when an untrusted notebook tries to run a cell that calls functions such as system or batch. functions is a comma-separated list of those Maxima function names">
          This notebook uses dangerous functions: <strong>{functions}</strong>. Trust it to
          allow execution.
        </Trans>
      </span>
      <button className="trust-banner-btn trust-btn" onClick={handleTrust}>
        {t({
          message: "Trust Notebook",
          comment: "Trust banner button: marks the notebook as trusted so its cells may call dangerous functions",
        })}
      </button>
      <button className="trust-banner-btn dismiss-btn" onClick={handleDismiss}>
        {t({ message: "Dismiss", comment: "Trust banner button: hides the banner without trusting the notebook" })}
      </button>
    </div>
  );
}
