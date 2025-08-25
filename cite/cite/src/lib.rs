use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, Expr, ItemFn, ItemImpl, ItemStruct, ItemTrait, ItemMod,
    Lit, Result, punctuated::Punctuated, Token,
};
mod mock;


/// The main `#[cite]` attribute macro
/// 
/// Supports struct reconstruction syntax like:
/// - `#[cite(MockSource::same("content"))]`
/// - `#[cite(MockSource::same("content"), reason = "why this is important")]`
/// - `#[cite(MockSource::same("content"), level = "WARN", annotation = "ANY")]`
/// 
/// Supports macro-style syntax like:
/// - `#[cite(mock(same("content")))]`
/// - `#[cite(mock(changed("a", "b")))]`
/// - `#[cite(mock(same("content"), reason = "why this is important"))]`
/// - `#[cite(mock(changed("a", "b"), level = "WARN", annotation = "ANY"))]`
#[proc_macro_attribute]
pub fn cite(args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse as a list of expressions separated by commas
    let args = parse_macro_input!(args with Punctuated::<Expr, Token![,]>::parse_terminated);
    
    // Parse the citation arguments
    let citation = match parse_citation_args_from_exprs(args) {
        Ok(citation) => citation,
        Err(err) => return err.to_compile_error().into(),
    };
    
    // Parse the item being annotated
    let input_clone = input.clone();
    
    // Try to parse as different item types
    if let Ok(item_fn) = syn::parse::<ItemFn>(input_clone.clone()) {
        return handle_function_citation(citation, item_fn).into();
    }
    
    if let Ok(item_struct) = syn::parse::<ItemStruct>(input_clone.clone()) {
        return handle_struct_citation(citation, item_struct).into();
    }
    
    if let Ok(item_trait) = syn::parse::<ItemTrait>(input_clone.clone()) {
        return handle_trait_citation(citation, item_trait).into();
    }
    
    if let Ok(item_impl) = syn::parse::<ItemImpl>(input_clone.clone()) {
        return handle_impl_citation(citation, item_impl).into();
    }
    
    if let Ok(item_mod) = syn::parse::<ItemMod>(input_clone) {
        return handle_mod_citation(citation, item_mod).into();
    }
    
    // If we can't parse it as a known item type, return an error
    syn::Error::new_spanned(
        proc_macro2::TokenStream::from(input),
        "cite attribute can only be applied to functions, structs, traits, impl blocks, or modules"
    ).to_compile_error().into()
}

/// Represents a parsed citation with all its attributes
#[derive(Clone)]
struct Citation {
    source_expr: Expr,
    reason: Option<String>,
    level: Option<String>,
    annotation: Option<String>,
}

/// Parse the citation arguments from expressions
fn parse_citation_args_from_exprs(args: Punctuated<Expr, Token![,]>) -> Result<Citation> {
    if args.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "cite attribute requires a source expression"
        ));
    }
    
    let mut source_expr = None;
    let mut reason = None;
    let mut level = None;
    let mut annotation = None;
    let mut first = true;
    
    for arg in args {
        match &arg {
            // First argument should be the source expression 
            _ if first => {
                source_expr = Some(arg);
                first = false;
            }
            
            // Handle named arguments like reason = "...", level = "WARN", etc.
            Expr::Assign(assign_expr) => {
                if let Expr::Path(path_expr) = &*assign_expr.left {
                    if let Some(ident) = path_expr.path.get_ident() {
                        let name = ident.to_string();
                        
                        match name.as_str() {
                            "reason" => {
                                if let Expr::Lit(expr_lit) = &*assign_expr.right {
                                    if let Lit::Str(lit_str) = &expr_lit.lit {
                                        reason = Some(lit_str.value());
                                    } else {
                                        return Err(syn::Error::new_spanned(&assign_expr.right, "reason must be a string literal"));
                                    }
                                } else {
                                    return Err(syn::Error::new_spanned(&assign_expr.right, "reason must be a string literal"));
                                }
                            }
                            "level" => {
                                if let Expr::Lit(expr_lit) = &*assign_expr.right {
                                    if let Lit::Str(lit_str) = &expr_lit.lit {
                                        level = Some(lit_str.value());
                                    } else {
                                        return Err(syn::Error::new_spanned(&assign_expr.right, "level must be a string literal"));
                                    }
                                } else {
                                    return Err(syn::Error::new_spanned(&assign_expr.right, "level must be a string literal"));
                                }
                            }
                            "annotation" => {
                                if let Expr::Lit(expr_lit) = &*assign_expr.right {
                                    if let Lit::Str(lit_str) = &expr_lit.lit {
                                        annotation = Some(lit_str.value());
                                    } else {
                                        return Err(syn::Error::new_spanned(&assign_expr.right, "annotation must be a string literal"));
                                    }
                                } else {
                                    return Err(syn::Error::new_spanned(&assign_expr.right, "annotation must be a string literal"));
                                }
                            }
                            _ => {
                                return Err(syn::Error::new_spanned(&path_expr.path, format!("Unknown citation attribute: {}", name)));
                            }
                        }
                    } else {
                        return Err(syn::Error::new_spanned(&assign_expr.left, "Expected identifier"));
                    }
                } else {
                    return Err(syn::Error::new_spanned(&assign_expr.left, "Expected identifier"));
                }
            }
            
            _ => {
                return Err(syn::Error::new_spanned(arg, "Unsupported citation argument format"));
            }
        }
    }
    
    let source_expr = source_expr.ok_or_else(|| {
        syn::Error::new(proc_macro2::Span::call_site(), "Missing source expression")
    })?;
    
    Ok(Citation {
        source_expr,
        reason,
        level,
        annotation,
    })
}

/// Handle citation on a function
fn handle_function_citation(citation: Citation, mut item_fn: ItemFn) -> proc_macro2::TokenStream {
    // Generate validation code that runs at compile time
    let validation_code = generate_validation_code(&citation);
    
    // Insert the validation as a const block at the beginning of the function
    let validation_stmt: syn::Stmt = parse_quote! {
        const _: () = { #validation_code };
    };
    
    item_fn.block.stmts.insert(0, validation_stmt);
    
    quote! { #item_fn }
}

/// Handle citation on a struct
fn handle_struct_citation(citation: Citation, item_struct: ItemStruct) -> proc_macro2::TokenStream {
    let validation_code = generate_validation_code(&citation);
    let struct_name = &item_struct.ident;
    let validation_const_name = syn::Ident::new(
        &format!("_CITE_VALIDATION_{}", struct_name),
        proc_macro2::Span::call_site()
    );
    
    quote! {
        #item_struct
        
        const #validation_const_name: () = { #validation_code };
    }
}

/// Handle citation on a trait
fn handle_trait_citation(citation: Citation, item_trait: ItemTrait) -> proc_macro2::TokenStream {
    let validation_code = generate_validation_code(&citation);
    let trait_name = &item_trait.ident;
    let validation_const_name = syn::Ident::new(
        &format!("_CITE_VALIDATION_{}", trait_name),
        proc_macro2::Span::call_site()
    );
    
    quote! {
        #item_trait
        
        const #validation_const_name: () = { #validation_code };
    }
}

/// Handle citation on an impl block
fn handle_impl_citation(citation: Citation, item_impl: ItemImpl) -> proc_macro2::TokenStream {
    let validation_code = generate_validation_code(&citation);
    
    // Generate a unique const name for this impl block
    let validation_const_name = syn::Ident::new(
        &format!("_CITE_VALIDATION_IMPL_{}", 
                 std::ptr::addr_of!(item_impl) as usize),
        proc_macro2::Span::call_site()
    );
    
    quote! {
        #item_impl
        
        const #validation_const_name: () = { #validation_code };
    }
}

/// Handle citation on a module
fn handle_mod_citation(citation: Citation, item_mod: ItemMod) -> proc_macro2::TokenStream {
    let validation_code = generate_validation_code(&citation);
    let mod_name = &item_mod.ident;
    let validation_const_name = syn::Ident::new(
        &format!("_CITE_VALIDATION_MOD_{}", mod_name),
        proc_macro2::Span::call_site()
    );
    
    quote! {
        #item_mod
        
        const #validation_const_name: () = { #validation_code };
    }
}

/// Generate validation code that executes the user's source expression with the real API
fn generate_validation_code(citation: &Citation) -> proc_macro2::TokenStream {
    // Actually try to perform validation during macro expansion
    let validation_result = attempt_macro_expansion_validation(citation);
    let source_expr = &citation.source_expr;
    
    let reason_comment = if let Some(_reason) = &citation.reason {
        quote! {
            // Citation reason: #_reason
        }
    } else {
        quote! {}
    };
    
    // Generate a unique function name to ensure the source import is used
    let use_source_fn_name = syn::Ident::new(
        &format!("_cite_use_source_{}", 
                 std::ptr::addr_of!(*citation) as usize),
        proc_macro2::Span::call_site()
    );
    
    // Generate code based on the validation result from macro expansion
    match validation_result {
        Ok(None) => {
            // Validation passed
            quote! {
                #reason_comment
                // Citation validation passed
                const _: () = ();
                
                // Include source to avoid unused import warnings
                #[allow(dead_code)]
                fn #use_source_fn_name() {
                    let _source = #source_expr;
                }
            }
        }
        Ok(Some(warning_msg)) => {
            // Validation failed but should only warn
            quote! {
                #reason_comment
                // Citation validation warning
                const _: () = {
                    const _WARNING: &str = #warning_msg;
                    ()
                };
                
                // Include source to avoid unused import warnings
                #[allow(dead_code)]
                fn #use_source_fn_name() {
                    let _source = #source_expr;
                }
            }
        }
        Err(error_msg) => {
            // Validation failed and should error
            let error_tokens = syn::Error::new(proc_macro2::Span::call_site(), error_msg).to_compile_error();
            quote! {
                #error_tokens
                
                // Include source to avoid unused import warnings even when erroring
                #[allow(dead_code)]
                fn #use_source_fn_name() {
                    let _source = #source_expr;
                }
            }
        }
    }
}

/// Attempt to perform validation during macro expansion
/// 
/// This is the key function that tries to execute the user's source expression
/// during macro expansion and return the validation result.
fn attempt_macro_expansion_validation(citation: &Citation) -> std::result::Result<Option<String>, String> {
    use cite_core::{CitationBehavior, CitationLevel};
    
    // Parse level override if provided
    let level_override = if let Some(level_str) = &citation.level {
        match level_str.as_str() {
            "ERROR" | "error" => Some(CitationLevel::Error),
            "WARN" | "warn" => Some(CitationLevel::Warn),
            "SILENT" | "silent" => Some(CitationLevel::Silent),
            _ => None,
        }
    } else {
        None
    };
    
    // Load behavior from environment (with defaults)
    let behavior = CitationBehavior::from_env();
    
    // Here's the challenge: we need to execute the user's source expression
    // during macro expansion. Since we can't directly eval arbitrary expressions,
    // we have a few options:
    // 
    // 1. Support specific known source patterns (like what I had before)
    // 2. Use a plugin system where sources register macro-expansion handlers
    // 3. Generate runtime validation and accept that errors happen at runtime
    // 4. Provide const-compatible source implementations
    //
    // For now, let's go with option 1 but make it more general by supporting
    // any source that implements a "macro expansion" trait or pattern
    
    // Try to handle common source patterns
    if let Some(result) = try_execute_source_expression(citation, &behavior, level_override) {
        return result;
    }
    
    // If we can't execute the source during macro expansion, assume it's valid
    // The user can always add explicit validation later
    Ok(None)
}

/// Try to execute source expressions that we can handle during macro expansion
fn try_execute_source_expression(
    citation: &Citation, 
    behavior: &cite_core::CitationBehavior, 
    level_override: Option<cite_core::CitationLevel>
) -> Option<std::result::Result<Option<String>, String>> {
    // Try to construct and execute MockSource using the mock module
    if let Some(mock_source) = mock::try_construct_mock_source_from_expr(&citation.source_expr) {
        use cite_core::Source;
        
        // Execute the real API!
        match mock_source.get() {
            Ok(comparison) => {
                let result = comparison.validate(behavior, level_override);
                
                if !result.is_valid() {
                    let diff_msg = format!(
                        "Citation content has changed!\n  Referenced: {}\n  Current: {}", 
                        comparison.referenced().0,
                        comparison.current().0
                    );
                    
                    if result.should_fail_compilation() {
                        return Some(Err(diff_msg));
                    } else if result.should_report() {
                        return Some(Ok(Some(diff_msg)));
                    }
                }
                
                return Some(Ok(None));
            }
            Err(e) => {
                return Some(Err(format!("Citation source error: {:?}", e)));
            }
        }
    }
    
    // Add support for other source types here as needed
    // For example: try_construct_http_source, try_construct_file_source, etc.
    
    None
}




