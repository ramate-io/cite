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
	ItemMod, ItemStruct, ItemTrait, Lit, Result, Token,
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
mod level;

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
mod mock;

/// HTTP/Http source parsing and construction
///
/// This module handles the parsing of HTTP source syntax and construction of
/// HttpMatch instances during macro expansion. It implements the keyword
/// argument parsing for HTTP sources.
///
/// The HTTP source syntax supports:
/// - `http, url = "https://example.com", pattern = "regex"`: Regex content extraction
/// - `http, url = "https://example.com", selector = "h1"`: CSS selector extraction  
/// - `http, url = "https://example.com", match_type = "full"`: Full document validation
///
/// Additional behavior parameters are handled by the main citation parser.
mod http;

mod git;

/// The main `#[cite]` attribute macro
///
/// Supports keyword argument syntax like:
/// - `#[cite(mock, same = "content")]`
/// - `#[cite(mock, changed = ("old", "new"))]`
/// - `#[cite(mock, same = "content", reason = "why this is important")]`
/// - `#[cite(mock, changed = ("old", "new"), level = "ERROR", annotation = "ANY")]`
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
	// For keyword syntax, store the raw arguments
	raw_args: Option<Vec<Expr>>,
}

/// Parse the citation arguments from expressions
fn parse_citation_args_from_exprs(args: Punctuated<Expr, Token![,]>) -> Result<Citation> {
	if args.is_empty() {
		return Err(syn::Error::new(
			proc_macro2::Span::call_site(),
			"cite attribute requires a source expression",
		));
	}

	let args_vec: Vec<_> = args.into_iter().collect();

	// Check if this uses the new keyword syntax starting with source type
	if let Some(first_arg) = args_vec.first() {
		if let Expr::Path(path_expr) = first_arg {
			if path_expr.path.segments.len() == 1 {
				let source_type = &path_expr.path.segments[0].ident.to_string();
				match source_type.as_str() {
					"mock" | "http" | "git" => {
						return parse_keyword_syntax(args_vec);
					}
					_ => {
						// Unknown source type, fall back to single expression syntax
					}
				}
			}
		}
	}

	// Fall back to the old single source expression syntax
	parse_single_source_syntax(args_vec)
}

/// Parse the new keyword syntax: SOURCE_TYPE, PARAMS..., level = "ERROR"
///
/// Supports both mock and http source types:
/// - mock, same = "content", level = "ERROR"
/// - http, url = "https://example.com", pattern = "regex", level = "WARN"
fn parse_keyword_syntax(args: Vec<Expr>) -> Result<Citation> {
	let mut reason = None;
	let mut level = None;
	let mut annotation = None;
	let mut source_args_found = false;

	// Parse all arguments, looking for source specification and other attributes
	for arg in &args {
		if let Expr::Assign(assign_expr) = arg {
			if let Expr::Path(left_path) = &*assign_expr.left {
				if left_path.path.segments.len() == 1 {
					let name = &left_path.path.segments[0].ident.to_string();

					match name.as_str() {
						// Mock source parameters
						"same" | "changed" => {
							source_args_found = true;
						}
						// HTTP source parameters
						"url" | "pattern" | "selector" | "match_type" | "fragment" | "cache" => {
							source_args_found = true;
						}
						"reason" => {
							if let Expr::Lit(expr_lit) = &*assign_expr.right {
								if let Lit::Str(lit_str) = &expr_lit.lit {
									reason = Some(lit_str.value());
								} else {
									return Err(syn::Error::new_spanned(
										&assign_expr.right,
										"reason must be a string literal",
									));
								}
							} else {
								return Err(syn::Error::new_spanned(
									&assign_expr.right,
									"reason must be a string literal",
								));
							}
						}
						"level" => {
							if let Expr::Lit(expr_lit) = &*assign_expr.right {
								if let Lit::Str(lit_str) = &expr_lit.lit {
									level = Some(lit_str.value());
								} else {
									return Err(syn::Error::new_spanned(
										&assign_expr.right,
										"level must be a string literal",
									));
								}
							} else {
								return Err(syn::Error::new_spanned(
									&assign_expr.right,
									"level must be a string literal",
								));
							}
						}
						"annotation" => {
							if let Expr::Lit(expr_lit) = &*assign_expr.right {
								if let Lit::Str(lit_str) = &expr_lit.lit {
									annotation = Some(lit_str.value());
								} else {
									return Err(syn::Error::new_spanned(
										&assign_expr.right,
										"annotation must be a string literal",
									));
								}
							} else {
								return Err(syn::Error::new_spanned(
									&assign_expr.right,
									"annotation must be a string literal",
								));
							}
						}
						_ => {
							return Err(syn::Error::new_spanned(
								&left_path.path,
								format!("Unknown citation attribute: {}", name),
							));
						}
					}
				}
			}
		}
	}

	if !source_args_found {
		return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "keyword syntax requires source-specific parameters (e.g., 'same = \"content\"' for mock or 'url = \"...\"' for http)"
        ));
	}

	// Now validate that the source type can actually be constructed with the given arguments
	// Try to construct the source to validate completeness
	if let Some(first_arg) = args.first() {
		if let Expr::Path(path_expr) = first_arg {
			if path_expr.path.segments.len() == 1 {
				let source_type = &path_expr.path.segments[0].ident.to_string();
				match source_type.as_str() {
					"mock" => {
						if mock::try_construct_mock_source_from_citation_args(&args).is_none() {
							return Err(syn::Error::new(
                                proc_macro2::Span::call_site(),
                                "invalid mock syntax - requires 'same = \"content\"' or 'changed = (\"old\", \"new\")'"
                            ));
						}
					}
					"http" => {
						if http::try_construct_http_source_from_citation_args(&args).is_none() {
							return Err(syn::Error::new(
                                proc_macro2::Span::call_site(),
                                "invalid http syntax - requires 'url = \"...\"' and one of 'pattern = \"...\"', 'selector = \"...\"', or 'match_type = \"full\"'"
                            ));
						}
					}
					_ => {
						// Unknown source type should have been caught earlier
					}
				}
			}
		}
	}

	// Create a special marker source expression with the args embedded
	// This allows us to pass the keyword arguments to the validation phase
	let source_expr = syn::parse_quote! { keyword_syntax };

	Ok(Citation { source_expr, reason, level, annotation, raw_args: Some(args) })
}

/// Parse the traditional single source expression syntax
fn parse_single_source_syntax(args: Vec<Expr>) -> Result<Citation> {
	let mut source_expr = None;
	let mut reason = None;
	let mut level = None;
	let mut annotation = None;
	let mut first = true;

	let args_clone = args.clone();
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
										return Err(syn::Error::new_spanned(
											&assign_expr.right,
											"reason must be a string literal",
										));
									}
								} else {
									return Err(syn::Error::new_spanned(
										&assign_expr.right,
										"reason must be a string literal",
									));
								}
							}
							"level" => {
								if let Expr::Lit(expr_lit) = &*assign_expr.right {
									if let Lit::Str(lit_str) = &expr_lit.lit {
										level = Some(lit_str.value());
									} else {
										return Err(syn::Error::new_spanned(
											&assign_expr.right,
											"level must be a string literal",
										));
									}
								} else {
									return Err(syn::Error::new_spanned(
										&assign_expr.right,
										"level must be a string literal",
									));
								}
							}
							"annotation" => {
								if let Expr::Lit(expr_lit) = &*assign_expr.right {
									if let Lit::Str(lit_str) = &expr_lit.lit {
										annotation = Some(lit_str.value());
									} else {
										return Err(syn::Error::new_spanned(
											&assign_expr.right,
											"annotation must be a string literal",
										));
									}
								} else {
									return Err(syn::Error::new_spanned(
										&assign_expr.right,
										"annotation must be a string literal",
									));
								}
							}
							_ => {
								return Err(syn::Error::new_spanned(
									&path_expr.path,
									format!("Unknown citation attribute: {}", name),
								));
							}
						}
					} else {
						return Err(syn::Error::new_spanned(
							&assign_expr.left,
							"Expected identifier",
						));
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

	Ok(Citation { source_expr, reason, level, annotation, raw_args: Some(args_clone) })
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
	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	quote! {
		#item_struct

		const #validation_const_name: () = { #validation_code };
	}
}

/// Handle citation on a trait
fn handle_trait_citation(citation: Citation, item_trait: ItemTrait) -> proc_macro2::TokenStream {
	let validation_code = generate_validation_code(&citation);
	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	quote! {
		#item_trait

		const #validation_const_name: () = { #validation_code };
	}
}

/// Handle citation on an impl block
fn handle_impl_citation(citation: Citation, item_impl: ItemImpl) -> proc_macro2::TokenStream {
	let validation_code = generate_validation_code(&citation);

	// Use counter for unique const name
	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	quote! {
		#item_impl

		const #validation_const_name: () = { #validation_code };
	}
}

/// Handle citation on a module
fn handle_mod_citation(citation: Citation, item_mod: ItemMod) -> proc_macro2::TokenStream {
	let validation_code = generate_validation_code(&citation);
	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	quote! {
		#item_mod

		const #validation_const_name: () = { #validation_code };
	}
}

/// Handle citation on an enum
fn handle_enum_citation(citation: Citation, item_enum: ItemEnum) -> proc_macro2::TokenStream {
	let validation_code = generate_validation_code(&citation);

	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	quote! {
		#item_enum

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

	// Generate code based on the validation result from macro expansion
	match validation_result {
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
	}
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
	if let Some(result) = try_execute_source_expression(citation, &behavior, level_override) {
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

/// Try to execute source expressions that we can handle during macro expansion
fn try_execute_source_expression(
	citation: &Citation,
	behavior: &cite_core::CitationBehavior,
	level_override: Option<cite_core::CitationLevel>,
) -> Option<std::result::Result<Option<String>, String>> {
	// Check if this uses keyword syntax by looking for the keyword_syntax marker
	if let Expr::Path(path_expr) = &citation.source_expr {
		if path_expr.path.segments.len() == 1
			&& path_expr.path.segments[0].ident == "keyword_syntax"
		{
			// Use keyword syntax parsing - try all source types
			if let Some(args) = &citation.raw_args {
				// Try mock sources first
				if let Some(mock_source) = mock::try_construct_mock_source_from_citation_args(args)
				{
					return execute_mock_source_validation(mock_source, behavior, level_override);
				}

				// Try HTTP sources
				if let Some(http_source) = http::try_construct_http_source_from_citation_args(args)
				{
					return execute_http_source_validation(http_source, behavior, level_override);
				}

				// Try Git sources
				if let Some(git_source) = git::try_construct_git_source_from_citation_args(args) {
					return execute_git_source_validation(git_source, behavior, level_override);
				}
			}
		}
	}

	// Try to construct and execute MockSource using the traditional expression parsing
	if let Some(mock_source) = mock::try_construct_mock_source_from_expr(&citation.source_expr) {
		return execute_mock_source_validation(mock_source, behavior, level_override);
	}

	// Add support for other source types here as needed
	None
}

/// Execute mock source validation and return the result
fn execute_mock_source_validation(
	mock_source: cite_core::mock::MockSource,
	behavior: &cite_core::CitationBehavior,
	level_override: Option<cite_core::CitationLevel>,
) -> Option<std::result::Result<Option<String>, String>> {
	use cite_core::Source;

	// Execute the real API!
	match mock_source.get() {
		Ok(comparison) => {
			let result = comparison.validate(behavior, level_override);

			if !result.is_valid() {
				let diff_msg = format!(
					"Citation content has changed!\n         Referenced: {}\n         Current: {}",
					comparison.referenced().0,
					comparison.current().0
				);

				if result.should_fail_compilation() {
					return Some(Err(diff_msg));
				} else if result.should_report() {
					return Some(Ok(Some(diff_msg)));
				}
			}

			Some(Ok(None))
		}
		Err(e) => Some(Err(format!("Citation source error: {:?}", e))),
	}
}

/// Execute HTTP source validation and return the result
fn execute_http_source_validation(
	http_source: cite_http::HttpMatch,
	behavior: &cite_core::CitationBehavior,
	level_override: Option<cite_core::CitationLevel>,
) -> Option<std::result::Result<Option<String>, String>> {
	use cite_core::Source;

	// HTTP sources now handle caching internally
	match http_source.get() {
		Ok(comparison) => {
			let result = comparison.validate(behavior, level_override);

			if !result.is_valid() {
				let diff_msg = if let Some(unified_diff) = comparison.diff().unified_diff() {
					format!(
						"HTTP citation content has changed!\n         URL: {}\n{}",
						comparison.current().source_url.as_str(),
						unified_diff
					)
				} else {
					format!(
                        "HTTP citation content has changed!\n         URL: {}\n         Current: {}\n         Referenced: {}",
                        comparison.current().source_url.as_str(),
                        comparison.current().content,
                        comparison.referenced().content
                    )
				};

				if result.should_fail_compilation() {
					return Some(Err(diff_msg));
				} else if result.should_report() {
					return Some(Ok(Some(diff_msg)));
				}
			}

			Some(Ok(None))
		}
		Err(e) => Some(Err(format!("HTTP citation source error: {:?}", e))),
	}
}

/// Execute Git source validation and return the result
fn execute_git_source_validation(
	git_source: cite_git::GitSource,
	behavior: &cite_core::CitationBehavior,
	level_override: Option<cite_core::CitationLevel>,
) -> Option<std::result::Result<Option<String>, String>> {
	use cite_core::{Diff, Source};

	// Git sources handle git operations internally
	match git_source.get() {
		Ok(comparison) => {
			let result = comparison.validate(behavior, level_override);

			if !result.is_valid() {
				let diff_msg = if let Some(unified_diff) = comparison.diff().unified_diff() {
					format!(
						"Git citation content has changed!\n         Remote: {}\n         Path: {}\n         Revision: {}\n{}",
						comparison.current().remote,
						comparison.current().path_pattern.path,
						comparison.current().revision,
						unified_diff
					)
				} else {
					format!(
                        "Git citation content has changed!\n         Remote: {}\n         Path: {}\n         Revision: {}",
                        comparison.current().remote,
                        comparison.current().path_pattern.path,
                        comparison.current().revision
                    )
				};

				if result.should_fail_compilation() {
					return Some(Err(diff_msg));
				} else if result.should_report() {
					return Some(Ok(Some(diff_msg)));
				}
			}

			Some(Ok(None))
		}
		Err(e) => Some(Err(format!("Git citation source error: {:?}", e))),
	}
}
