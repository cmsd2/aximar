import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nProvider } from "@lingui/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ label: "main" }) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }));
vi.mock("./PlotlyChart", () => ({ PlotlyChart: () => null }));

import { i18n } from "../lib/i18n";
import { mapSyncCells } from "../hooks/useNotebookEvents";
import type { SyncCell } from "../lib/notebook-commands";
import { CellOutput } from "./CellOutput";

/** A cell as the backend sends it after a run that timed out after 30 seconds */
const timedOut: SyncCell = {
  id: "c1",
  cell_type: "code",
  input: "for i:1 thru 10^9 do x: i$",
  status: "error",
  output: {
    text_output: "",
    latex: null,
    plot_svg: null,
    plot_data: null,
    image_png: null,
    error: "Evaluation timed out after 30 seconds",
    is_error: true,
    duration_ms: 0,
    output_label: null,
    execution_count: 3,
    text_plain: null,
    stale: false,
    error_info: {
      id: "error.timeout",
      params: { seconds: "30" },
      messages: {
        title: { id: "error.timeout.title", template: "Timed Out" },
        explanation: {
          id: "error.timeout.explanation",
          template: "The cell ran for longer than {seconds} seconds, so Aximar stopped it.",
        },
      },
      title: "Timed Out",
      explanation: "The cell ran for longer than 30 seconds, so Aximar stopped it.",
      suggestion: null,
      example: null,
      did_you_mean: [],
      correct_signatures: [],
    },
  },
};

describe("a cell that timed out", () => {
  it("shows why in its output", () => {
    const [cell] = mapSyncCells([timedOut]);
    expect(cell.status).toBe("error");
    const html = renderToStaticMarkup(
      <I18nProvider i18n={i18n}>
        <CellOutput output={cell.output!} cellId="c1" />
      </I18nProvider>,
    );
    expect(html).toContain("Timed Out");
    expect(html).toContain("The cell ran for longer than 30 seconds, so Aximar stopped it.");
  });
});
