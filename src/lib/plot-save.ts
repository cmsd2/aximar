// Saving a plot to a file: a gnuplot SVG as it is, or a Plotly chart as an
// SVG or PNG image of what's drawn, or as its JSON.  The output's hover
// toolbar and the plot viewer both use these.
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { savePlotFile } from "./notebooks-client";

/** Ask where to save an image of `format` */
async function askPath(i18n: I18n, format: "svg" | "png"): Promise<string | null> {
  const kind = format.toUpperCase();
  return save({
    defaultPath: `plot.${format}`,
    filters: [
      {
        name: i18n._(
          msg({
            message: `${kind} Image`,
            comment: "File type in the Save dialog when saving a plot. kind is the image format, SVG or PNG",
          }),
        ),
        extensions: [format],
      },
    ],
  });
}

/** Save a plot's SVG text, as gnuplot drew it */
export async function savePlotSvg(i18n: I18n, svg: string): Promise<void> {
  const path = await askPath(i18n, "svg");
  if (path) await invoke("write_plot_svg", { path, content: svg });
}

/** Save an image of the Plotly chart drawn in `el`, at its size on screen */
export async function savePlotlyImage(i18n: I18n, el: HTMLElement, format: "svg" | "png"): Promise<void> {
  const path = await askPath(i18n, format);
  if (!path) return;
  // Loaded when needed: Plotly needs a browser, and most outputs aren't charts
  const { default: Plotly } = await import("plotly.js-dist-min");
  const data = await Plotly.toImage(el, { format, width: el.clientWidth || 800, height: el.clientHeight || 500 });
  if (format === "svg") {
    const svg = data.startsWith("data:") ? decodeURIComponent(data.split(",")[1]) : data;
    await invoke("write_plot_svg", { path, content: svg });
  } else {
    const bytes = Uint8Array.from(atob(data.split(",")[1]), (c) => c.charCodeAt(0));
    await invoke("write_binary_file", { path, data: Array.from(bytes) });
  }
}

/** Save a Plotly chart's JSON */
export async function savePlotlyJson(plotData: string): Promise<void> {
  await savePlotFile(plotData, null);
}
