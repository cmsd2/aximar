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
fn test_ax_draw2d_vector_field() {
    let (stdout, _) = run_maxima(
        "ax_draw2d(ax_vector_field(-y, x, x, -3, 3, y, -3, 3));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 2, "vector field produces shaft + head traces");
    assert_eq!(data[0]["type"], "scatter");
    assert_eq!(data[0]["mode"], "lines");
    assert_eq!(data[1]["type"], "scatter");
    assert_eq!(data[1]["mode"], "markers");
}

#[test]
#[ignore]
fn test_ax_draw2d_vector_field_normalized() {
    let (stdout, _) = run_maxima(
        "ax_draw2d(ax_vector_field(x, y, x, -2, 2, y, -2, 2), normalize=true, ngrid=5);",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 2);
    // Head trace should have triangle-up markers
    assert_eq!(data[1]["marker"]["symbol"], "triangle-up");
}

#[test]
#[ignore]
fn test_ax_draw2d_streamline() {
    let (stdout, _) = run_maxima(
        "ax_draw2d(ax_streamline(-y, x, x, -3, 3, y, -3, 3));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert!(data.len() >= 1, "streamline should produce at least one trace");
    for trace in data {
        assert_eq!(trace["type"], "scatter");
        assert_eq!(trace["mode"], "lines");
    }
}

#[test]
#[ignore]
fn test_ax_draw2d_streamline_custom_initial_points() {
    let (stdout, _) = run_maxima(
        "ax_draw2d(initial_points=[[1,0],[0,1]], ax_streamline(-y, x, x, -3, 3, y, -3, 3));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 2, "two initial points -> two trajectory traces");
}

#[test]
#[ignore]
fn test_ax_draw2d_phase_portrait() {
    let (stdout, _) = run_maxima(
        r#"ax_draw2d(color="gray", ax_vector_field(-y, x, x, -3, 3, y, -3, 3), color="red", ax_streamline(-y, x, x, -3, 3, y, -3, 3));"#,
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert!(data.len() >= 3, "should have vector field traces + streamline traces");
}

// ── Numerical correctness tests ──────────────────────────────────────────

/// Extract a labeled float value from Maxima stdout, e.g. "MAX_ERR: 1.23e-12"
fn parse_maxima_value(stdout: &str, label: &str) -> f64 {
    let joined = stdout.replace("\\\n", "");
    for line in joined.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(label) {
            let val_str = rest.trim();
            // Maxima may print "1.23e-12" or "1.23E-12" or just "0.0"
            return val_str
                .parse::<f64>()
                .unwrap_or_else(|e| panic!("failed to parse '{val_str}' as f64: {e}"));
        }
    }
    panic!("label '{label}' not found in Maxima output:\n{joined}");
}

#[test]
#[ignore]
fn test_rk4_conservation_rotation_system() {
    // dx/dt = -y, dy/dt = x has exact invariant x² + y² = const.
    // Starting at (1, 0), all trajectory points should satisfy x² + y² ≈ 1.
    let (stdout, _) = run_maxima(
        "ax_draw2d(initial_points=[[1,0]], t_range=[0, 2*float(%pi)], dt=0.01, ax_streamline(-y, x, x, -3, 3, y, -3, 3));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1, "one initial point -> one trajectory");

    let xs = data[0]["x"].as_array().unwrap();
    let ys = data[0]["y"].as_array().unwrap();
    assert!(xs.len() > 100, "should have many trajectory points");

    let mut max_deviation = 0.0_f64;
    for (xv, yv) in xs.iter().zip(ys.iter()) {
        let x = xv.as_f64().unwrap();
        let y = yv.as_f64().unwrap();
        let r_sq = x * x + y * y;
        let deviation = (r_sq - 1.0).abs();
        max_deviation = max_deviation.max(deviation);
    }
    assert!(
        max_deviation < 1e-6,
        "x² + y² should be conserved to 1e-6, but max deviation = {max_deviation}"
    );
}

#[test]
#[ignore]
fn test_rk4_endpoint_accuracy_full_circle() {
    // Rotation system from (1, 0) with t = 2π should return near (1, 0).
    // With dt=0.01, ceil(2π/0.01) = 629 steps → t_final = 6.29, overshooting 2π by ~0.0068.
    // So the endpoint distance from (1,0) is bounded by dt (the max overshoot).
    let (stdout, _) = run_maxima(
        "ax_draw2d(initial_points=[[1,0]], t_range=[0, 2*float(%pi)], dt=0.01, ax_streamline(-y, x, x, -3, 3, y, -3, 3));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    let xs = data[0]["x"].as_array().unwrap();
    let ys = data[0]["y"].as_array().unwrap();

    // Full path = reverse(backward) + rest(forward).
    // Both endpoints overshoot ±2π by at most dt, so distance from (1, 0) ≤ dt.
    let x_first = xs.first().unwrap().as_f64().unwrap();
    let y_first = ys.first().unwrap().as_f64().unwrap();
    let x_last = xs.last().unwrap().as_f64().unwrap();
    let y_last = ys.last().unwrap().as_f64().unwrap();

    let dist_first = ((x_first - 1.0).powi(2) + y_first.powi(2)).sqrt();
    let dist_last = ((x_last - 1.0).powi(2) + y_last.powi(2)).sqrt();

    assert!(
        dist_first < 0.01 && dist_last < 0.01,
        "both endpoints should be within dt of (1, 0).\n\
         first=({x_first:.6}, {y_first:.6}) dist={dist_first:.6}\n\
         last=({x_last:.6}, {y_last:.6}) dist={dist_last:.6}"
    );
}

#[test]
#[ignore]
fn test_rk4_cross_validate_with_maxima_rk() {
    // Compare our ax__rk4_trajectory against Maxima's built-in rk() from dynamics package.
    // Both are RK4 with identical step size — should agree to near machine precision.
    // Use t_final = 6.0 (exact multiple of dt=0.01) to avoid last-step differences
    // (Maxima's rk may take a shorter final step to hit the endpoint exactly, while
    // ours always takes full steps).
    let (stdout, stderr) = run_maxima(
        r#"load("dynamics")$
our: ax__rk4_trajectory(-y, x, x, y, 1, 0, 0, 6.0, 0.01, -5, 5, -5, 5)$
ref: rk([-y, x], [x, y], [1, 0], [t, 0, 6.0, 0.01])$
n: min(length(our), length(ref))$
max_err: lmax(makelist(
  sqrt((our[i][1] - ref[i][2])^2 + (our[i][2] - ref[i][3])^2),
  i, 1, n))$
print("MAX_ERR:", max_err)$"#,
    );
    assert!(
        !stderr.contains("error"),
        "Maxima should not produce errors: {stderr}"
    );
    let max_err = parse_maxima_value(&stdout, "MAX_ERR:");
    assert!(
        max_err < 1e-10,
        "our RK4 and Maxima's rk() should agree to 1e-10, but max error = {max_err}"
    );
}

#[test]
#[ignore]
fn test_rk4_nonlinear_cross_validate_lotka_volterra() {
    // Cross-validate on a nonlinear system: Lotka-Volterra dx/dt = x(1-y), dy/dt = y(x-1)
    let (stdout, stderr) = run_maxima(
        r#"load("dynamics")$
our: ax__rk4_trajectory(x*(1-y), y*(x-1), x, y, 1.5, 1.5, 0, 5, 0.01, -10, 10, -10, 10)$
ref: rk([x*(1-y), y*(x-1)], [x, y], [1.5, 1.5], [t, 0, 5, 0.01])$
n: min(length(our), length(ref))$
max_err: lmax(makelist(
  sqrt((our[i][1] - ref[i][2])^2 + (our[i][2] - ref[i][3])^2),
  i, 1, n))$
print("MAX_ERR:", max_err)$"#,
    );
    assert!(
        !stderr.contains("error"),
        "Maxima should not produce errors: {stderr}"
    );
    let max_err = parse_maxima_value(&stdout, "MAX_ERR:");
    assert!(
        max_err < 1e-10,
        "our RK4 and Maxima's rk() should agree on Lotka-Volterra, but max error = {max_err}"
    );
}

#[test]
#[ignore]
fn test_vector_field_arrow_directions_uniform() {
    // Uniform rightward field F = (1, 0): all arrows should point right (angle ≈ 0).
    // Plotly angle formula: 90 - atan2(dy, dx) * 180/π = 90 - atan2(0, 1) * 180/π = 90 - 0 = 90
    // Wait — for F=(1,0), atan2(0, positive) = 0, so angle = 90 - 0 = 90... but that's wrong.
    // Actually, Plotly's triangle-up points +y by default, and marker.angle rotates clockwise.
    // So angle=0 means pointing up, angle=90 means pointing right. Our formula gives 90° for rightward. ✓
    let (stdout, _) = run_maxima(
        "ax_draw2d(ngrid=5, ax_vector_field(1, 0, x, -1, 1, y, -1, 1));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    let head_trace = &data[1]; // second trace is arrowheads
    let angles = head_trace["marker"]["angle"].as_array().unwrap();

    for (i, angle_val) in angles.iter().enumerate() {
        let angle = angle_val.as_f64().unwrap();
        // For F=(1, 0): atan2(0, 1) = 0, so angle = 90 - 0 = 90°
        assert!(
            (angle - 90.0).abs() < 1e-6,
            "arrow {i} should have angle ≈ 90° (rightward), got {angle}"
        );
    }
}

#[test]
#[ignore]
fn test_vector_field_diagonal_direction() {
    // Field F = (1, 1) at every point: arrows should point at 45° from +x.
    // atan2(1, 1) = π/4, angle = 90 - 45 = 45°
    let (stdout, _) = run_maxima(
        "ax_draw2d(ngrid=3, ax_vector_field(1, 1, x, -1, 1, y, -1, 1));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    let head_trace = &data[1];
    let angles = head_trace["marker"]["angle"].as_array().unwrap();

    for (i, angle_val) in angles.iter().enumerate() {
        let angle = angle_val.as_f64().unwrap();
        // atan2(1, 1) = 45°, Plotly angle = 90 - 45 = 45°
        assert!(
            (angle - 45.0).abs() < 1e-6,
            "arrow {i} should have angle ≈ 45°, got {angle}"
        );
    }
}

#[test]
#[ignore]
fn test_streamline_exponential_growth() {
    // dx/dt = x, dy/dt = 0 from (1, 1): exact solution x(t) = e^t, y(t) = 1.
    // After t=1: x ≈ e ≈ 2.718, y = 1.
    let (stdout, _) = run_maxima(
        "ax_draw2d(initial_points=[[1,1]], t_range=[0, 1], dt=0.01, ax_streamline(x, 0, x, -10, 10, y, -3, 3));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1, "one trajectory");

    let xs = data[0]["x"].as_array().unwrap();
    let ys = data[0]["y"].as_array().unwrap();

    // All y values should be ≈ 1
    for (i, yv) in ys.iter().enumerate() {
        let y = yv.as_f64().unwrap();
        assert!(
            (y - 1.0).abs() < 1e-10,
            "y[{i}] should be 1.0 (no y dynamics), got {y}"
        );
    }

    // The forward integration goes from (1,1) to (e, 1).
    // The backward integration goes from (1,1) toward (e^-1, 1) ≈ (0.368, 1).
    // Full path = reverse(backward) + rest(forward), so:
    // first x ≈ e^-1 ≈ 0.368, last x ≈ e ≈ 2.718
    let x_first = xs.first().unwrap().as_f64().unwrap();
    let x_last = xs.last().unwrap().as_f64().unwrap();

    let e = std::f64::consts::E;
    let e_inv = 1.0 / e;

    assert!(
        (x_last - e).abs() < 1e-4,
        "last x should be ≈ e ≈ 2.718, got {x_last}"
    );
    assert!(
        (x_first - e_inv).abs() < 1e-4,
        "first x should be ≈ 1/e ≈ 0.368, got {x_first}"
    );
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

#[test]
#[ignore]
fn test_ax_draw3d_parametric_curve() {
    let (stdout, _) = run_maxima(
        "ax_draw3d(parametric(cos(t), sin(t), t/5, t, 0, 4*%pi));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["type"], "scatter3d");
    assert_eq!(data[0]["mode"], "lines");
    assert!(data[0]["x"].as_array().unwrap().len() > 10);
    assert!(data[0]["z"].as_array().unwrap().len() > 10);
}

#[test]
#[ignore]
fn test_ax_draw3d_parametric_surface() {
    let (stdout, _) = run_maxima(
        "ax_draw3d(parametric_surface(cos(u)*sin(v), sin(u)*sin(v), cos(v), u, 0, 2*%pi, v, 0, %pi));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["type"], "surface");
    // x, y, z should all be 2D matrices
    let x = data[0]["x"].as_array().unwrap();
    assert!(!x.is_empty());
    assert!(x[0].is_array(), "x should be a 2D matrix for parametric_surface");
    let y = data[0]["y"].as_array().unwrap();
    assert!(y[0].is_array(), "y should be a 2D matrix for parametric_surface");
    let z = data[0]["z"].as_array().unwrap();
    assert!(z[0].is_array());
}

#[test]
#[ignore]
fn test_ax_draw3d_implicit() {
    let (stdout, _) = run_maxima(
        "ax_draw3d(implicit(x^2 + y^2 + z^2 = 1, x, -1.5, 1.5, y, -1.5, 1.5, z, -1.5, 1.5));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["type"], "isosurface");
    assert_eq!(data[0]["isomin"], 0);
    assert_eq!(data[0]["isomax"], 0);
    // flat arrays for x, y, z, value
    let x = data[0]["x"].as_array().unwrap();
    let value = data[0]["value"].as_array().unwrap();
    assert!(!x.is_empty());
    assert_eq!(x.len(), value.len());
}

#[test]
#[ignore]
fn test_ax_draw3d_contour() {
    let (stdout, _) = run_maxima(
        "ax_draw3d(ax_contour3d(x^2 - y^2, x, -2, 2, y, -2, 2));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["type"], "surface");
    // Should have contours.z.show = true
    assert_eq!(data[0]["contours"]["z"]["show"], true);
    assert_eq!(data[0]["contours"]["z"]["project"]["z"], true);
}

#[test]
#[ignore]
fn test_ax_draw3d_vector_field() {
    let (stdout, _) = run_maxima(
        "ax_draw3d(ax_vector_field3d(-y, x, 0, x, -2, 2, y, -2, 2, z, -1, 1));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["type"], "cone");
    // Should have flat arrays for x, y, z, u, v, w
    let x = data[0]["x"].as_array().unwrap();
    let u = data[0]["u"].as_array().unwrap();
    assert!(!x.is_empty());
    assert_eq!(x.len(), u.len());
}

#[test]
#[ignore]
fn test_ax_draw2d_box() {
    let (stdout, _) = run_maxima(
        "ax_draw2d(boxpoints=\"all\", boxmean=\"sd\", ax_box(makelist(random(100)/10.0, i, 1, 30)));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["type"], "box");
    let y = data[0]["y"].as_array().unwrap();
    assert_eq!(y.len(), 30);
    assert_eq!(data[0]["boxpoints"], "all");
    assert_eq!(data[0]["boxmean"], "sd");
}

#[test]
#[ignore]
fn test_ax_draw2d_violin() {
    let (stdout, _) = run_maxima(
        "ax_draw2d(ax_violin(makelist(random(100)/10.0, i, 1, 50), \"Group A\"));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["type"], "violin");
    let y = data[0]["y"].as_array().unwrap();
    assert_eq!(y.len(), 50);
    assert_eq!(data[0]["box"]["visible"], true);
    assert_eq!(data[0]["meanline"]["visible"], true);
}

#[test]
#[ignore]
fn test_ax_draw2d_pie() {
    let (stdout, _) = run_maxima(
        "ax_draw2d(hole=0.4, ax_pie([40, 30, 20, 10], [\"A\", \"B\", \"C\", \"D\"]));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["type"], "pie");
    let values = data[0]["values"].as_array().unwrap();
    assert_eq!(values.len(), 4);
    let labels = data[0]["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 4);
    assert_eq!(labels[0], "A");
    let hole = data[0]["hole"].as_f64().unwrap();
    assert!((hole - 0.4).abs() < 1e-6);
}

#[test]
#[ignore]
fn test_ax_draw2d_error_bar() {
    let (stdout, _) = run_maxima(
        "ax_draw2d(ax_error_bar([1,2,3,4], [2.1,3.9,6.2,7.8], [0.3,0.5,0.2,0.4], [0.1,0.15,0.1,0.2]));",
    );
    let json = extract_plotly_json(&stdout).expect("should produce plotly JSON");
    let spec = parse_plotly(&json);

    let data = spec["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["type"], "scatter");
    assert_eq!(data[0]["mode"], "markers");
    // y error bars
    assert_eq!(data[0]["error_y"]["visible"], true);
    let y_err = data[0]["error_y"]["array"].as_array().unwrap();
    assert_eq!(y_err.len(), 4);
    // x error bars
    assert_eq!(data[0]["error_x"]["visible"], true);
    let x_err = data[0]["error_x"]["array"].as_array().unwrap();
    assert_eq!(x_err.len(), 4);
}
