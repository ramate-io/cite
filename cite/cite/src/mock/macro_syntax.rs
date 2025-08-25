use syn::{Expr, Lit};
use cite_core::mock::MockSource;

/// Parse the macro-style syntax: mock(changed("a", "b")) and mock(same("content"))
/// 
/// This syntax provides a more concise, macro-like DSL for specifying mock sources
/// within citation attributes.
pub fn try_parse(expr: &Expr) -> Option<MockSource> {
    if let Expr::Call(call_expr) = expr {
        if let Expr::Path(path_expr) = &*call_expr.func {
            let path_str = quote::quote!(#path_expr).to_string();
            
            // Handle mock(...)
            if path_str == "mock" && call_expr.args.len() == 1 {
                if let Expr::Call(inner_call) = &call_expr.args[0] {
                    if let Expr::Path(inner_path) = &*inner_call.func {
                        let inner_path_str = quote::quote!(#inner_path).to_string();
                        
                        // Handle mock(changed("a", "b"))
                        if inner_path_str == "changed" && inner_call.args.len() == 2 {
                            return parse_changed_args(&inner_call.args);
                        }
                        
                        // Handle mock(same("content"))
                        if inner_path_str == "same" && inner_call.args.len() == 1 {
                            return parse_same_args(&inner_call.args);
                        }
                    }
                }
            }
        }
    }
    
    None
}

/// Parse arguments for mock(changed("referenced", "current"))
fn parse_changed_args(args: &syn::punctuated::Punctuated<Expr, syn::Token![,]>) -> Option<MockSource> {
    let args: Vec<_> = args.iter().collect();
    if let (Expr::Lit(lit1), Expr::Lit(lit2)) = (args[0], args[1]) {
        if let (Lit::Str(str1), Lit::Str(str2)) = (&lit1.lit, &lit2.lit) {
            return Some(MockSource::changed(str1.value(), str2.value()));
        }
    }
    None
}

/// Parse arguments for mock(same("content"))
fn parse_same_args(args: &syn::punctuated::Punctuated<Expr, syn::Token![,]>) -> Option<MockSource> {
    if let Expr::Lit(lit) = &args[0] {
        if let Lit::Str(str_lit) = &lit.lit {
            return Some(MockSource::same(str_lit.value()));
        }
    }
    None
}