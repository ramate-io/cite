use syn::Expr;
use cite_util_core::mock::MockSource;

mod macro_syntax;
mod struct_syntax;

/// Try to construct a MockSource from the citation expression using various syntax patterns
/// 
/// This function handles parsing of MockSource constructor expressions and creates
/// the appropriate MockSource during macro expansion.
pub fn try_construct_mock_source_from_expr(expr: &Expr) -> Option<MockSource> {
    // Parse macro-style syntax: mock(changed("a", "b")) and mock(same("content"))
    if let Some(mock_source) = macro_syntax::try_parse(expr) {
        return Some(mock_source);
    }
    
    // Parse struct-style syntax: MockSource::changed("a", "b") and MockSource::same("content")
    if let Some(mock_source) = struct_syntax::try_parse(expr) {
        return Some(mock_source);
    }
    
    None
}
