use super::*;

fn fixture(name: &str) -> String {
    let path = format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn json(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn cell<'a>(file: &'a NotebookFile, input: &str) -> &'a Cell {
    file.cells.iter().find(|c| c.input == input).unwrap_or_else(|| panic!("no cell {input:?}"))
}

#[test]
fn the_spec_example_round_trips_byte_for_byte() {
    let file = read(&fixture("spec-example.macnb")).unwrap();
    assert!(!file.ids_replaced);
    assert_eq!(write(&file), fixture("spec-example.macnb"));
}

#[test]
fn written_files_are_nbformat_4_5_and_end_with_a_newline() {
    let file = read(&fixture("aximar-window-before-s1.macnb")).unwrap();
    let text = write(&file);
    assert!(text.ends_with("}\n"));
    let v = json(&text);
    assert_eq!((v["nbformat"].as_u64(), v["nbformat_minor"].as_u64()), (Some(4), Some(5)));
    assert!(v["cells"].as_array().unwrap().iter().all(|c| c["id"].is_string()));
    assert_eq!(v["metadata"]["kernelspec"]["name"], "maxima");
}

#[test]
fn cell_ids_follow_the_rule() {
    let file = read(&fixture("cell-ids.macnb")).unwrap();
    let ids: Vec<&str> = file.cells.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids[0], "a1");
    assert_ne!(ids[1], "a1", "a duplicate gets a new ID");
    assert_eq!(ids[2], "b-2_c");
    for replaced in &ids[3..] {
        assert!(valid_id(replaced) && replaced.len() == 21, "{replaced}");
    }
    assert!(file.ids_replaced);
    let unique: HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len());
}

#[test]
fn new_ids_are_valid_and_distinct() {
    let ids: HashSet<String> = (0..1000).map(|_| new_id()).collect();
    assert_eq!(ids.len(), 1000);
    assert!(ids.iter().all(|id| valid_id(id) && id.len() == 21));
}

#[test]
fn every_loaded_output_is_stale() {
    for name in ["spec-example.macnb", "vscode-extension-before-s1.macnb", "aximar-mcp-before-s1.macnb"] {
        let file = read(&fixture(name)).unwrap();
        for out in file.cells.iter().filter_map(|c| c.output.as_ref()) {
            assert!(out.stale, "{name}");
        }
    }
}

#[test]
fn the_vscode_file_reads_plots_as_plots_and_keeps_errors() {
    let file = read(&fixture("vscode-extension-before-s1.macnb")).unwrap();
    // The text/plain copy of the SVG source is not shown (#6)
    let plot = cell(&file, "plot2d(x^2, [x, 0, 1])$").output.as_ref().unwrap();
    assert!(plot.plot_svg.is_some());
    assert!(plot.text_output.is_empty(), "{:?}", plot.text_output);
    let plotly = cell(&file, "plot2d(x, [x, 0, 1], [plot_format, plotly])$").output.as_ref().unwrap();
    assert!(plotly.plot_data.is_some() && plotly.text_output.is_empty());
    // Print text written as a text/plain bundle is print text
    let result = cell(&file, "print(\"working\")$\nintegrate(x, x);");
    let out = result.output.as_ref().unwrap();
    assert_eq!(out.text_output, "working");
    assert_eq!(out.latex.as_deref(), Some("{{x^2}\\over{2}}"));
    // The extension's own metadata is kept
    assert_eq!(result.metadata["outputLabel"], "%o8");
    // Its error output is an error (#7)
    let out = cell(&file, "1/0;").output.as_ref().unwrap();
    assert!(out.is_error);
    assert!(out.error.as_deref().unwrap().starts_with("expt: undefined"));
}

#[test]
fn the_jupyter_file_keeps_raw_cells_metadata_and_plain_results() {
    let file = read(&fixture("jupyter-4.0.ipynb")).unwrap();
    assert!(file.ids_replaced);
    assert_eq!(file.cells[0].cell_type, CellType::Raw);
    assert_eq!(file.metadata["jupytext"]["formats"], "ipynb,md");
    let code = cell(&file, "x: 2$\nx^2;");
    assert_eq!(code.metadata["tags"], json(r#"["setup"]"#));
    let out = code.output.as_ref().unwrap();
    assert_eq!(out.text_plain.as_deref(), Some("4"));
    assert_eq!(out.execution_count, Some(5));
    // Written back, everything but the new IDs and the version is the same
    let mut original = json(&fixture("jupyter-4.0.ipynb"));
    let mut written = json(&write(&file));
    for v in [&mut original, &mut written] {
        v["nbformat_minor"] = json!(0);
        for c in v["cells"].as_array_mut().unwrap() {
            c.as_object_mut().unwrap().remove("id");
            // Sources are written as lines
            c["source"] = json!(text(Some(&c["source"])));
        }
    }
    assert_eq!(written, original);
}

#[test]
fn files_saved_before_s1_open() {
    let file = read(&fixture("aximar-mcp-before-s1.macnb")).unwrap();
    assert!(cell(&file, "plot2d(x^2, [x, 0, 1])$").output.as_ref().unwrap().plot_svg.is_some());
    // The failed cell was saved with no outputs (#7)
    assert!(cell(&file, "1/0;").output.is_none());
    let file = read(&fixture("aximar-window-before-s1.macnb")).unwrap();
    assert_eq!(cell(&file, "print(\"working\")$\nintegrate(x, x);").output.as_ref().unwrap().text_output, "working");
}

#[test]
fn an_error_in_the_print_text_is_written_once() {
    let file = NotebookFile {
        cells: vec![Cell {
            id: "c".into(),
            cell_type: CellType::Code,
            input: "x x;".into(),
            metadata: Map::new(),
            extra: Map::new(),
            output: Some(Output {
                text_output: "incorrect syntax: x is not an infix operator\n".into(),
                is_error: true,
                execution_count: Some(1),
                ..Default::default()
            }),
        }],
        ..Default::default()
    };
    let v = json(&write(&file));
    let outputs = v["cells"][0]["outputs"].as_array().unwrap();
    assert_eq!(outputs.len(), 1, "{outputs:?}");
    assert_eq!(outputs[0]["output_type"], "error");
    assert_eq!(outputs[0]["evalue"], "incorrect syntax: x is not an infix operator");
}

#[test]
fn unknown_outputs_and_cell_fields_are_kept() {
    let text = r#"{"nbformat": 4, "nbformat_minor": 5, "metadata": {}, "cells": [
        {"id": "c", "cell_type": "code", "source": "1;", "metadata": {}, "execution_count": 1, "outputs": [
            {"output_type": "display_data", "data": {"text/html": ["<b>x</b>"]}, "metadata": {}},
            {"output_type": "future_type", "payload": 1}
        ]},
        {"id": "m", "cell_type": "markdown", "source": "![x](attachment:x.png)", "metadata": {},
         "attachments": {"x.png": {"image/png": "iVBORw0KGgo="}}}]}"#;
    let v = json(&write(&read(text).unwrap()));
    let outputs = v["cells"][0]["outputs"].as_array().unwrap();
    assert_eq!(outputs[0]["data"]["text/html"], json(r#"["<b>x</b>"]"#));
    assert_eq!(outputs[1]["output_type"], "future_type");
    assert_eq!(v["cells"][1]["attachments"]["x.png"]["image/png"], "iVBORw0KGgo=");
}

#[test]
fn other_versions_are_refused() {
    assert!(matches!(read(r#"{"nbformat": 3, "cells": []}"#), Err(FormatError::Version(3))));
    assert!(matches!(read("not json"), Err(FormatError::Json(_))));
    assert!(matches!(read("[]"), Err(FormatError::Shape(_))));
}

#[test]
fn the_model_survives_a_trip_through_serde() {
    // The WebAssembly build passes the model to JavaScript and back this way
    let file = read(&fixture("spec-example.macnb")).unwrap();
    let back: NotebookFile = serde_json::from_value(serde_json::to_value(&file).unwrap()).unwrap();
    assert_eq!(back, file);
    assert_eq!(serde_json::to_value(&file).unwrap()["cells"][0]["cellType"], "markdown");
}

#[test]
#[ignore = "rewrites fixtures/spec-example.macnb in the writer's own layout"]
fn regenerate_spec_example() {
    let path = format!("{}/fixtures/spec-example.macnb", env!("CARGO_MANIFEST_DIR"));
    let before = fixture("spec-example.macnb");
    let written = write(&read(&before).unwrap());
    assert_eq!(json(&written), json(&before), "only the layout may change");
    std::fs::write(path, written).unwrap();
}
