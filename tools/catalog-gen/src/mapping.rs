use aximar_lib::catalog::types::FunctionCategory;

/// Map Maxima's fine-grained category strings to our FunctionCategory enum.
///
/// Maxima's `<defcategory>` has two kinds of values:
/// - **Topical** (e.g., "Differential calculus") — maps to a specific category
/// - **Type-based** (e.g., "Function", "Option variable") — not a topical category,
///   maps to `Other` silently
///
/// Logs truly unmapped categories to stderr when `log_unmapped` is true.
pub fn map_category(maxima_category: &str, log_unmapped: bool) -> FunctionCategory {
    let lower = maxima_category.to_lowercase();

    // --- Topical categories ---

    // Calculus (chapters: Differentiation, Integration, Differential Equations,
    //   Sums Products and Series [partial], Limits [partial])
    if contains_any(&lower, &["differential calculus", "integral calculus",
        "differential equations", "differentiation", "integration",
        "sums and products"]) {
        return FunctionCategory::Calculus;
    }

    // Algebra (chapters: Expressions, Evaluation, Data Types and Structures, Sets, Operators)
    if contains_any(&lower, &["algebraic manipulation", "expressions",
        "evaluation", "substitution", "sets", "lists",
        "data types and structures", "operators"]) {
        return FunctionCategory::Algebra;
    }

    // Linear Algebra (chapters: Matrices and Linear Algebra, Package linearalgebra, Package lapack)
    if contains_any(&lower, &["matrices and linear algebra", "matrices",
        "linear algebra", "package linearalgebra", "package lapack"]) {
        return FunctionCategory::LinearAlgebra;
    }

    // Simplification (chapters: Simplification, Rules and Patterns, Package simplification)
    if contains_any(&lower, &["simplification", "rational expressions",
        "rules and patterns"]) {
        return FunctionCategory::Simplification;
    }

    // Solving (chapters: Equations, Package to_poly_solve, Package mnewton)
    if contains_any(&lower, &["equations", "root finding", "solving equations",
        "roots", "package to_poly_solve", "package mnewton"]) {
        return FunctionCategory::Solving;
    }

    // Plotting (chapters: Plotting, Package draw, Package drawdf)
    if contains_any(&lower, &["plotting", "visualization",
        "package draw", "package drawdf"]) {
        return FunctionCategory::Plotting;
    }

    // Trigonometry (chapter: Package trigtools, Elementary Functions [partial])
    if contains_any(&lower, &["trigonometric", "trigonometry", "package trigtools"]) {
        return FunctionCategory::Trigonometry;
    }

    // Number Theory (chapters: Number Theory, Package combinatorics)
    if contains_any(&lower, &["number theory", "integers", "prime numbers",
        "combinatorics"]) {
        return FunctionCategory::NumberTheory;
    }

    // Polynomials (chapters: Polynomials, Package grobner, Package orthopoly)
    if contains_any(&lower, &["polynomials", "package grobner", "package orthopoly"]) {
        return FunctionCategory::Polynomials;
    }

    // Series (chapters: Sums Products and Series, Limits)
    if contains_any(&lower, &["taylor series", "power series", "limits",
        "series", "sums, products, and series"]) {
        return FunctionCategory::Series;
    }

    // Programming (chapters: Function Definition, Program Flow, Debugging)
    if contains_any(&lower, &["programming", "control flow",
        "function definition", "program flow", "debugging", "predicates"]) {
        return FunctionCategory::Programming;
    }

    // I/O (chapters: File Input and Output, Command Line, Package numericalio)
    if contains_any(&lower, &["file input and output", "input and output",
        "file i/o", "display", "package numericalio"]) {
        return FunctionCategory::IO;
    }

    // --- Definition-type categories (not topical — silently map to Other) ---
    if is_definition_type(&lower) {
        return FunctionCategory::Other;
    }

    // Truly unmapped
    if log_unmapped {
        eprintln!("unmapped category: {maxima_category:?}");
    }
    FunctionCategory::Other
}

/// Check if the category string is a definition type rather than a topical category.
fn is_definition_type(lower: &str) -> bool {
    matches!(lower,
        "function" | "functions" | "system function"
        | "option variable" | "optional variable"
        | "system variable" | "global variable" | "variable"
        | "special symbol" | "operator" | "infix operator" | "special operator"
        | "declaration" | "input terminator" | "property" | "symbol property"
        | "constant" | "keyword" | "package"
        | "graphic object" | "scene object" | "scene constructor"
    ) || lower.ends_with(" option")
}

fn contains_any(value: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| value.contains(p))
}
