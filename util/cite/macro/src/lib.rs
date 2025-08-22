use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, Expr, ItemFn, ItemImpl, ItemStruct, ItemTrait,
    Lit, Result, punctuated::Punctuated, Token,
};

/// The main `#[cite]` attribute macro
/// 
/// Supports syntax like:
/// - `#[cite(MockSource::same("content"))]`
/// - `#[cite(MockSource::same("content"), reason = "why this is important")]`
/// - `#[cite(MockSource::same("content"), level = "WARN", annotation = "ANY")]`
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
    
    if let Ok(item_impl) = syn::parse::<ItemImpl>(input_clone) {
        return handle_impl_citation(citation, item_impl).into();
    }
    
    // If we can't parse it as a known item type, return an error
    syn::Error::new_spanned(
        proc_macro2::TokenStream::from(input),
        "cite attribute can only be applied to functions, structs, traits, or impl blocks"
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

/// Generate the validation code that will run at compile time
fn generate_validation_code(citation: &Citation) -> proc_macro2::TokenStream {
    // For now, we'll generate a no-op validation that just preserves the citation info
    // In the future, this will:
    // 1. Validate sources during macro expansion (not in const context)
    // 2. Respect environment variables for error/warning levels
    // 3. Cache validation results
    
    let reason_comment = if let Some(reason) = &citation.reason {
        let _reason_text = format!("Citation reason: {}", reason);
        quote! {
            // Citation reason: #reason
        }
    } else {
        quote! {}
    };
    
    // Generate a zero-runtime-cost validation marker
    // This preserves citation metadata without running code at runtime
    quote! {
        #reason_comment
        
        // Citation marker - zero runtime cost
        // TODO: During macro expansion, validate the source and emit compile-time warnings/errors
        // based on environment variables and source comparison
        let _citation_marker: () = ();
        _citation_marker
    }
}
