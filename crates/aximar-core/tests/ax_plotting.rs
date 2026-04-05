//! Integration tests for ax_plotting.mac
//!
//! These tests require Maxima to be installed and on PATH.
//! They are marked `#[ignore]` so they don't run in CI by default.
//! Run with: `cargo test -p aximar-core --test ax_plotting -- --ignored`

use std::io::Write;
use std::process::{Command, Stdio};

const AX_PLOTTING_LISP: &str = include_str!("../src/maxima/ax_plotting.lisp");
const AX_PLOTTING_MAC: &str = include_str!("../src/maxima/ax_plotting.mac");

/// Write the .lisp and .mac files to a temp location and run a Maxima expression.
/// Returns (stdout, stderr).
fn run_maxima(expr: &str) -> (String, String) {
    let dir = tempfile::tempdir().expect("create temp dir");

    let lisp_path = dir.path().join("ax_plotting.lisp");
    std::fs::write(&lisp_path, AX_PLOTTING_LISP).expect("write .lisp file");
    let mac_path = dir.path().join("ax_plotting.mac");
    std::fs::write(&mac_path, AX_PLOTTING_MAC).expect("write .mac file");

    // Use stdin to pipe: Lisp init → Maxima load → expression
    let input = format!(
        ":lisp (load \"{}\")\nload(\"{}\")$ {}\n",
        lisp_path.display(),
        mac_path.display(),
        expr,
    );

    let mut child = Command::new("maxima")
        .arg("--very-quiet")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("maxima must be installed");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .expect("write to maxima stdin");

    let output = child.wait_with_output().expect("wait for maxima");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr)
}

/// Extract the .plotly.json file path from Maxima output and read the file.
fn extract_plotly_json(stdout: &str) -> Option<String> {
    // Maxima wraps long lines with `\` + newline — join them first
    let joined = stdout.replace("\\\n", "");
    for line in joined.lines() {
        let trimmed = line.trim().trim_matches('"');
        if trimmed.ends_with(".plotly.json") {
            if let Ok(content) = std::fs::read_to_string(trimmed) {
                return Some(content);
            }
        }
    }
    None
}

fn parse_plotly(json_str: &str) -> serde_json::Value {
    serde_json::from_str(json_str).expect("plotly JSON must be valid")
}

// ── Tests ────────────────────────────────────────────────────────────────

#[test]
#[ignore]
fn test_mac_file_loads() {
    let (stdout, stderr) = run_maxima("1 + 1;");
    assert!(
        !stdout.contains("incorrect syntax") && !stderr.contains("incorrect syntax"),
        "ax_plotting.mac should load without syntax errors.\nstdout: {stdout}\nstderr: {stderr}"
    );
}

#[test]
#[ignore]
fn test_ax_plot2d_sin() {
    let (stdout, _) = run_maxima("ax_plot2d(sin(x), [x, -%pi, %pi]);");
    let json = extract_plotly_json(&stdout).expect("should produce a .plotly.json file");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().expect("data should be an array");
    assert_eq!(data.len(), 1, "single expression -> one trace");

    let trace = &data[0];
    assert_eq!(trace["type"], "scatter");
    assert_eq!(trace["mode"], "lines");

    let xs = trace["x"].as_array().expect("x should be array");
    let ys = trace["y"].as_array().expect("y should be array");
    assert_eq!(xs.len(), ys.len());
    assert!(xs.len() > 10, "should have many sample points");
}

#[test]
#[ignore]
fn test_ax_plot2d_multiple_expressions() {
    let (stdout, _) = run_maxima("ax_plot2d([sin(x), cos(x)], [x, -5, 5]);");
    let json = extract_plotly_json(&stdout).expect("should produce a .plotly.json file");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().expect("data should be an array");
    assert_eq!(data.len(), 2, "two expressions -> two traces");
}

#[test]
#[ignore]
fn test_ax_draw2d_explicit() {
    let (stdout, _) = run_maxima(
        r#"ax_draw2d(color="red", explicit(x^2, x, -3, 3));"#,
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);

    let trace = &data[0];
    assert_eq!(trace["type"], "scatter");
    assert_eq!(trace["line"]["color"].as_str().unwrap(), "red");
}

#[test]
#[ignore]
fn test_ax_draw2d_parametric() {
    let (stdout, _) = run_maxima("ax_draw2d(parametric(cos(t), sin(t), t, 0, 2*%pi));");
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["type"], "scatter");
    assert_eq!(data[0]["mode"], "lines");
}

#[test]
#[ignore]
fn test_ax_draw2d_points() {
    let (stdout, _) = run_maxima("ax_draw2d(points([[1,1],[2,4],[3,9]]));");
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["type"], "scatter");
    assert_eq!(data[0]["mode"], "markers");
    assert_eq!(data[0]["x"].as_array().unwrap().len(), 3);
}

#[test]
#[ignore]
fn test_ax_draw2d_implicit() {
    let (stdout, _) = run_maxima("ax_draw2d(implicit(x^2 + y^2 = 4, x, -3, 3, y, -3, 3));");
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["type"], "contour");
}

#[test]
#[ignore]
fn test_ax_draw2d_multiple_traces() {
    let (stdout, _) = run_maxima(
        r#"ax_draw2d(color="red", explicit(sin(x), x, -%pi, %pi), color="blue", explicit(cos(x), x, -%pi, %pi));"#,
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 2, "two explicit objects -> two traces");
    assert_eq!(data[0]["line"]["color"].as_str().unwrap(), "red");
    assert_eq!(data[1]["line"]["color"].as_str().unwrap(), "blue");
}

#[test]
#[ignore]
fn test_ax_draw2d_layout_options() {
    let (stdout, _) = run_maxima(
        r#"ax_draw2d(explicit(x, x, 0, 1), title="My Title", xlabel="X", ylabel="Y");"#,
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    assert_eq!(spec["layout"]["title"]["text"], "My Title");
    assert_eq!(spec["layout"]["xaxis"]["title"]["text"], "X");
    assert_eq!(spec["layout"]["yaxis"]["title"]["text"], "Y");
}

#[test]
#[ignore]
fn test_ax_draw3d_surface() {
    let (stdout, _) = run_maxima(
        "ax_draw3d(explicit(sin(x)*cos(y), x, -%pi, %pi, y, -%pi, %pi));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["type"], "surface");
    let z = data[0]["z"].as_array().unwrap();
    assert!(!z.is_empty());
    assert!(z[0].is_array());
}

#[test]
#[ignore]
fn test_ax_draw3d_points() {
    let (stdout, _) = run_maxima("ax_draw3d(points([[1,1,1],[2,2,4],[3,3,9]]));");
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["type"], "scatter3d");
    assert_eq!(data[0]["mode"], "markers");
}

#[test]
#[ignore]
fn test_valid_json_output() {
    let (stdout, _) = run_maxima("ax_plot2d(x^2, [x, -1, 1]);");
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec: serde_json::Value = serde_json::from_str(&json).expect("must be valid JSON");
    assert!(spec.get("data").is_some(), "must have 'data' field");
    assert!(spec.get("layout").is_some(), "must have 'layout' field");
}

#[test]
#[ignore]
fn test_sampling_values_are_finite() {
    let (stdout, _) = run_maxima("ax_plot2d(sin(x), [x, 0, 1]);");
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let ys = spec["data"][0]["y"].as_array().unwrap();
    for y in ys {
        assert!(
            y.is_number(),
            "all y values should be finite numbers, got: {y}"
        );
    }
}

#[test]
#[ignore]
fn test_unique_filenames() {
    // Two consecutive plots should produce different file paths
    let (stdout, _) = run_maxima(
        "ax_plot2d(sin(x), [x, 0, 1]); ax_plot2d(cos(x), [x, 0, 1]);",
    );
    let joined = stdout.replace("\\\n", "");
    let paths: Vec<&str> = joined
        .lines()
        .map(|l| l.trim())
        .filter(|l| l.ends_with(".plotly.json"))
        .collect();
    assert_eq!(paths.len(), 2, "should produce two plotly files");
    assert_ne!(paths[0], paths[1], "file paths must be unique");
}
