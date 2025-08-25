use syn::{Expr, Lit};
use cite_core::mock::{MockSource, mock_source_same, mock_source_changed};

/// Parse the struct-style syntax: MockSource::changed("a", "b") and MockSource::same("content")
/// 
/// This syntax follows standard Rust constructor patterns and provides familiar
/// object-oriented style for specifying mock sources.
pub fn try_parse(expr: &Expr) -> Option<MockSource> {
    if let Expr::Call(call_expr) = expr {
        if let Expr::Path(path_expr) = &*call_expr.func {
            let path_str = quote::quote!(#path_expr).to_string();
            
            // Handle MockSource::changed("a", "b")
            if path_str.contains("MockSource :: changed") && call_expr.args.len() == 2 {
                return parse_changed_args(&call_expr.args);
            }
            
            // Handle MockSource::same("content")
            if path_str.contains("MockSource :: same") && call_expr.args.len() == 1 {
                return parse_same_args(&call_expr.args);
            }
        }
    }
    
    None
}

/// Parse arguments for MockSource::changed("referenced", "current")
fn parse_changed_args(args: &syn::punctuated::Punctuated<Expr, syn::Token![,]>) -> Option<MockSource> {
    let args: Vec<_> = args.iter().collect();
    if let (Expr::Lit(lit1), Expr::Lit(lit2)) = (args[0], args[1]) {
        if let (Lit::Str(str1), Lit::Str(str2)) = (&lit1.lit, &lit2.lit) {
            return Some(mock_source_changed(str1.value(), str2.value()));
        }
    }
    None
}

/// Parse arguments for MockSource::same("content")
fn parse_same_args(args: &syn::punctuated::Punctuated<Expr, syn::Token![,]>) -> Option<MockSource> {
    if let Expr::Lit(lit) = &args[0] {
        if let Lit::Str(str_lit) = &lit.lit {
            return Some(mock_source_same(str_lit.value()));
        }
    }
    None
}
