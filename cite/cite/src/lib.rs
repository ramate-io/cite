//! Cite - Compile-time citation validation for Rust
//!
//! This crate provides procedural macros for annotating Rust code with citations
//! and validating the referenced content at compile time.
//!
//! # Design Philosophy
//!
//! The cite system is built around several core design principles:
//!
//! ## 1. Zero Runtime Overhead
//!
//! All citation validation happens during macro expansion at compile time. This means:
//! - No runtime performance impact on your application
//! - Network calls and content validation occur during `cargo build`
//! - Validation results directly influence compilation success/failure
//!
//! ## 2. Keyword Argument Syntax
//!
//! After evaluating multiple syntax approaches, we settled on keyword arguments:
//!
//! ```rust,ignore
//! #[cite(mock, same = "content")]                    // ✅ Current approach
//! #[cite(mock(same("content")))]                     // ❌ Function call ambiguity
//! #[cite(MockSource::same("content"))]               // ❌ Complex AST patterns
//! ```
//!
//! The keyword syntax offers:
//! - **Unambiguous parsing**: No conflicts with Rust's expression grammar
//! - **Extensibility**: Easy to add new source types and parameters
//! - **Readability**: Natural flow from source to parameters to behavior
//!
//! ## 3. Modular Architecture
//!
//! The system is split into focused crates:
//! - `cite` (this crate): Procedural macro implementation, user-facing API
//! - `cite-core`: Core traits, types, and runtime utilities
//! - `cite-test`: Comprehensive test suite with compile-time behavior tests
//!
//! This separation allows:
//! - Lightweight core that can be used without macro overhead
//! - Clear separation of compile-time vs runtime concerns
//! - Easier testing and maintenance
//!
//! ## 4. Environment Integration
//!
//! Citations can be controlled globally via environment variables:
//! - `CITE_LEVEL`: Set global error/warning behavior
//! - `CITE_ANNOTATION`: Control annotation output format
//! - `CITE_GLOBAL`: Set strict vs lenient mode
//!
//! This enables different behavior in development vs CI vs production builds.
//!
//! # Syntax Evolution
//!
//! The citation syntax has evolved through several iterations:
//!
//! ## Evolution 1: Direct Source Construction
//! ```rust,ignore
//! #[cite(MockSource::same("content"))]
//! ```
//! **Problem**: Required complex AST pattern matching in the macro, limited to
//! known source types, couldn't handle arbitrary expressions.
//!
//! ## Evolution 2: Helper Macros
//! ```rust,ignore
//! #[cite(mock!(same!("content")))]
//! ```
//! **Problem**: Nested macro expansion order issues, the inner macros would
//! generate function calls that the outer macro couldn't parse reliably.
//!
//! ## Evolution 3: Function-like Syntax
//! ```rust,ignore
//! #[cite(mock(same("content")))]
//! ```
//! **Problem**: Rust's macro parser interpreted this as a function call, leading
//! to "cannot find function" errors and complex parsing ambiguities.
//!
//! ## Evolution 4: Keyword Arguments (Current)
//! ```rust,ignore
//! #[cite(mock, same = "content")]
//! #[cite(mock, changed = ("old", "new"), level = "ERROR")]
//! ```
//! **Success**: Clean separation between source type, source parameters, and
//! behavior parameters. No parsing ambiguities, easily extensible.
//!
//! # Implementation Details
//!
//! ## Macro Expansion Flow
//!
//! 1. **Parse Arguments**: Extract source type, source params, and behavior params
//! 2. **Construct Source**: Create source object using keyword argument parsing
//! 3. **Validate Content**: Execute source.get() during macro expansion
//! 4. **Generate Code**: Emit validation results as compile-time errors/warnings
//! 5. **Preserve Original**: Return the original item unchanged for runtime
//!
//! ## Validation Strategy
//!
//! The macro uses pattern matching to handle different source types:
//! ```rust,ignore
//! if args[0] == "mock" {
//!     parse_mock_source(&args[1..])
//! } else if args[0] == "http" {
//!     parse_http_source(&args[1..])  // Future
//! }
//! ```
//!
//! This allows extending to new source types without breaking existing code.
//!
//! ## Error Propagation
//!
//! Validation results are converted to compile-time diagnostics:
//! - `Ok(None)`: Validation passed, no output
//! - `Ok(Some(msg))`: Validation failed, emit warning
//! - `Err(msg)`: Validation failed, emit error and fail compilation
//!
//! The specific behavior depends on the `level` parameter and environment variables.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
	parse_macro_input, parse_quote, punctuated::Punctuated, Expr, ItemEnum, ItemFn, ItemImpl,
	ItemMod, ItemStruct, ItemTrait, Token,
};

// Counter for generating unique validation constant names
use std::sync::atomic::{AtomicUsize, Ordering};

static VALIDATION_COUNTER: AtomicUsize = AtomicUsize::new(0);
static SOURCE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_validation_id() -> usize {
	VALIDATION_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn next_source_id() -> usize {
	SOURCE_COUNTER.fetch_add(1, Ordering::SeqCst)
}

mod annotation;
mod documentation;
mod extraction;
mod level;
mod prevalidation;
mod sources;
mod validation;

/// Mock source parsing and construction
///
/// This module handles the parsing of mock source syntax and construction of
/// MockSource instances during macro expansion. It implements the keyword
/// argument parsing for mock sources.
///
/// The mock source syntax supports:
/// - `mock, same = "content"`: Content that should remain unchanged
/// - `mock, changed = ("old", "new")`: Content that has changed from old to new
///
/// Additional behavior parameters are handled by the main citation parser.

/// The main `#[cite]` attribute macro
///
/// Supports keyword argument syntax like:
/// - `#[cite(mock, same = "content")]`
/// - `#[cite(mock, changed = ("old", "new"))]`
/// - `#[cite(mock, same = "content", reason = "why this is important")]`
/// - `#[cite(mock, changed = ("old", "new"), level = "ERROR", annotation = "ANY")]`
#[proc_macro_attribute]
pub fn cite(args: TokenStream, input: TokenStream) -> TokenStream {
	// Parse the input item
	let input_clone = input.clone();
	let mut item = parse_macro_input!(input_clone as syn::Item);

	// Parse as a list of expressions separated by commas
	let args = parse_macro_input!(args with Punctuated::<Expr, Token![,]>::parse_terminated);
	let args_vec: Vec<_> = args.into_iter().collect();

	// Parse into key-value map
	let mut kwargs = std::collections::HashMap::new();

	// Parse the first argument as the source type
	if let Some(source_type) = extraction::extract_source_type(&args_vec) {
		kwargs.insert("src".to_string(), serde_json::Value::String(source_type));
	}

	// If source is "above", parse the doc attribute and remove it
	if kwargs.get("src").and_then(|v| v.as_str()) == Some("above") {
		match extraction::above::parse_above_into_kwargs(&mut item) {
			Ok(doc_kwargs) => {
				// Replace the "above" source with the actual source from doc comment
				if let Some(actual_src) = doc_kwargs.get("src") {
					kwargs.insert("src".to_string(), actual_src.clone());
				} else {
					return syn::Error::new(
						proc_macro2::Span::call_site(),
						"<cite above> block must contain a 'src' field",
					)
					.to_compile_error()
					.into();
				}
				// Merge all other doc kwargs
				for (key, value) in doc_kwargs {
					if key != "src" {
						kwargs.insert(key, value);
					}
				}
			}
			Err(err) => return err.to_compile_error().into(),
		}
	}

	// Parse remaining cite arguments into kwargs
	if args_vec.len() > 1 {
		let additional_kwargs = extraction::parse_cite_kwargs(&args_vec[1..]);
		for (key, value) in additional_kwargs {
			kwargs.insert(key, value);
		}
	}

	// Validate and create citation
	let citation = match prevalidation::validate_with_kwargs(&kwargs, &args_vec) {
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

	if let Ok(item_mod) = syn::parse::<ItemMod>(input_clone.clone()) {
		return handle_mod_citation(citation, item_mod).into();
	}

	if let Ok(item_enum) = syn::parse::<ItemEnum>(input_clone) {
		return handle_enum_citation(citation, item_enum).into();
	}

	// If we can't parse it as a known item type, return an error
	syn::Error::new_spanned(
        proc_macro2::TokenStream::from(input),
        "cite attribute can only be applied to functions, structs, enums, traits, impl blocks, or modules"
    ).to_compile_error().into()
}

/// Represents a parsed citation with all its attributes
#[derive(Clone, Debug)]
struct Citation {
	source_expr: Expr,
	reason: Option<String>,
	level: Option<String>,
	annotation: Option<String>,

	// For kwargs syntax, store the parsed kwargs
	kwargs: Option<std::collections::HashMap<String, serde_json::Value>>,
}

impl Citation {
	pub fn get_src(&self) -> Result<String, String> {
		if let Some(kwargs) = &self.kwargs {
			if let Some(src) = kwargs.get("src") {
				return Ok(src.as_str().ok_or("src must be a string")?.to_string());
			}
		}
		Err("src not found".to_string())
	}
}

/// Handle citation on a function
fn handle_function_citation(citation: Citation, mut item_fn: ItemFn) -> proc_macro2::TokenStream {
	// Generate validation code that runs at compile time
	let (warning_text, validation_code) = generate_validation_code(&citation);

	// Insert the validation as a const block at the beginning of the function
	let validation_stmt: syn::Stmt = parse_quote! {
		const _: () = { #validation_code };
	};

	item_fn.block.stmts.insert(0, validation_stmt);

	// Add citation footnote to doc comments
	documentation::add_citation_footnote_to_item(&mut item_fn.attrs, &citation, warning_text);

	quote! { #item_fn }
}

/// Handle citation on a struct
fn handle_struct_citation(
	citation: Citation,
	mut item_struct: ItemStruct,
) -> proc_macro2::TokenStream {
	let (warning_text, validation_code) = generate_validation_code(&citation);
	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	// Add citation footnote to doc comments
	documentation::add_citation_footnote_to_item(&mut item_struct.attrs, &citation, warning_text);

	quote! {
		#item_struct

		const #validation_const_name: () = { #validation_code };
	}
}

/// Handle citation on a trait
fn handle_trait_citation(
	citation: Citation,
	mut item_trait: ItemTrait,
) -> proc_macro2::TokenStream {
	let (warning_text, validation_code) = generate_validation_code(&citation);
	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	// Add citation footnote to doc comments
	documentation::add_citation_footnote_to_item(&mut item_trait.attrs, &citation, warning_text);

	quote! {
		#item_trait

		const #validation_const_name: () = { #validation_code };
	}
}

/// Handle citation on an impl block
fn handle_impl_citation(citation: Citation, mut item_impl: ItemImpl) -> proc_macro2::TokenStream {
	let (warning_text, validation_code) = generate_validation_code(&citation);

	// Use counter for unique const name
	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	// Add citation footnote to doc comments
	documentation::add_citation_footnote_to_item(&mut item_impl.attrs, &citation, warning_text);

	quote! {
		#item_impl

		const #validation_const_name: () = { #validation_code };
	}
}

/// Handle citation on a module
fn handle_mod_citation(citation: Citation, mut item_mod: ItemMod) -> proc_macro2::TokenStream {
	let (warning_text, validation_code) = generate_validation_code(&citation);
	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	// Add citation footnote to doc comments
	documentation::add_citation_footnote_to_item(&mut item_mod.attrs, &citation, warning_text);

	quote! {
		#item_mod

		const #validation_const_name: () = { #validation_code };
	}
}

/// Handle citation on an enum
fn handle_enum_citation(citation: Citation, mut item_enum: ItemEnum) -> proc_macro2::TokenStream {
	let (warning_text, validation_code) = generate_validation_code(&citation);

	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	// Add citation footnote to doc comments
	documentation::add_citation_footnote_to_item(&mut item_enum.attrs, &citation, warning_text);

	quote! {
		#item_enum

		const #validation_const_name: () = { #validation_code };
	}
}

/// Generate validation code that executes the user's source expression with the real API
/// Returns (warning_text, validation_code)
fn generate_validation_code(citation: &Citation) -> (String, proc_macro2::TokenStream) {
	let source_expr = &citation.source_expr;

	let reason_comment = if let Some(_reason) = &citation.reason {
		quote! {
			// Citation reason: #_reason
		}
	} else {
		quote! {}
	};

	// Check if this is keyword syntax - if so, don't generate a source usage function
	let is_keyword_syntax = if let Expr::Path(path_expr) = source_expr {
		path_expr.path.segments.len() == 1 && path_expr.path.segments[0].ident == "keyword_syntax"
	} else {
		false
	};

	// Generate a unique function name to ensure the source import is used (only for non-keyword syntax)
	let use_source_fn_name = syn::Ident::new(
		&format!("_cite_use_source_{}", next_source_id()),
		proc_macro2::Span::call_site(),
	);

	// Actually try to perform validation during macro expansion
	let validation_result = attempt_macro_expansion_validation(citation);
	let warning_text = match &validation_result {
		Ok(Some(warning)) => warning.clone(),
		_ => String::new(),
	};

	// Generate code based on the validation result from macro expansion
	let validation_code = match validation_result {
		Ok(None) => {
			// Validation passed
			if is_keyword_syntax {
				quote! {
					#reason_comment
					// Citation validation passed
					const _: () = ();
				}
			} else {
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
		}
		Ok(Some(warning_msg)) => {
			// Validation failed but should only warn
			if is_keyword_syntax {
				quote! {
					#reason_comment
					// Citation validation warning
					#[deprecated(note = #warning_msg)]
					const fn _citation_warning() {}
					const _: () = _citation_warning();
				}
			} else {
				quote! {
					#reason_comment
					// Citation validation warning
					#[deprecated(note = #warning_msg)]
					const fn _citation_warning() {}
					const _: () = _citation_warning();

					// Include source to avoid unused import warnings even when erroring
					#[allow(dead_code)]
					fn #use_source_fn_name() {
						let _source = #source_expr;
					}
				}
			}
		}
		Err(error_msg) => {
			// Validation failed and should error
			let error_tokens =
				syn::Error::new(proc_macro2::Span::call_site(), error_msg).to_compile_error();
			if is_keyword_syntax {
				quote! {
					#error_tokens
				}
			} else {
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
	};

	(warning_text, validation_code)
}

/// Attempt to perform validation during macro expansion
///
/// This is the key function that tries to execute the user's source expression
/// during macro expansion and return the validation result.
fn attempt_macro_expansion_validation(
	citation: &Citation,
) -> std::result::Result<Option<String>, String> {
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

	// Load behavior from feature flags
	let behavior = CitationBehavior::from_features();

	// Check annotation requirements first
	let annotation_result = annotation::check_annotation_requirements(citation, &behavior)?;

	// Try to handle common source patterns
	if let Some(result) =
		validation::try_execute_source_expression(citation, &behavior, level_override)
	{
		return match (result, annotation_result) {
			// if also an annotation result, join them together
			(Ok(Some(result)), Some(annotation_result)) => {
				Ok(Some(format!("{}\n{}", result, annotation_result)))
			}
			(Ok(Some(result)), None) => Ok(Some(result)),
			(Ok(None), Some(annotation_result)) => Ok(Some(annotation_result)),
			(Ok(None), None) => Ok(None),
			(Err(error), _) => Err(error),
		};
	}

	// If we can't execute the source during macro expansion, assume it's valid
	// The user can always add explicit validation later
	Ok(None)
}
