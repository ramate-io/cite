use syn::{Expr, Lit};
use cite_core::mock::MockSource;

/// Parse the keyword argument syntax for mock sources
/// 
/// Supports syntax like:
/// - `mock, same = "content"`
/// - `mock, changed = ("old", "new")`
pub fn try_parse_from_citation_args(args: &[Expr]) -> Option<MockSource> {
    // Look for pattern: mock, same = "content" or mock, changed = ("old", "new")
    if args.is_empty() {
        return None;
    }
    
    // First argument should be the identifier "mock"
    if let Expr::Path(path_expr) = &args[0] {
        if path_expr.path.segments.len() == 1 && path_expr.path.segments[0].ident == "mock" {
            // Look through remaining arguments for assignments
            for arg in &args[1..] {
                if let Expr::Assign(assign_expr) = arg {
                    if let Expr::Path(left_path) = &*assign_expr.left {
                        if left_path.path.segments.len() == 1 {
                            let name = &left_path.path.segments[0].ident;
                            
                            match name.to_string().as_str() {
                                "same" => {
                                    if let Some(content) = extract_string_literal(&assign_expr.right) {
                                        return Some(MockSource::same(content));
                                    }
                                }
                                "changed" => {
                                    if let Some((old, new)) = extract_string_tuple(&assign_expr.right) {
                                        return Some(MockSource::changed(old, new));
                                    }
                                }
                                _ => continue,
                            }
                        }
                    }
                }
            }
        }
    }
    
    None
}

/// Extract a string literal from an expression
fn extract_string_literal(expr: &Expr) -> Option<String> {
    if let Expr::Lit(lit_expr) = expr {
        if let Lit::Str(str_lit) = &lit_expr.lit {
            return Some(str_lit.value());
        }
    }
    None
}

/// Extract a tuple of two string literals from an expression
fn extract_string_tuple(expr: &Expr) -> Option<(String, String)> {
    if let Expr::Tuple(tuple_expr) = expr {
        if tuple_expr.elems.len() == 2 {
            let first = extract_string_literal(&tuple_expr.elems[0])?;
            let second = extract_string_literal(&tuple_expr.elems[1])?;
            return Some((first, second));
        }
    }
    None
}