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
	// Parse the input item
	let input_clone = input.clone();
	let mut item = parse_macro_input!(input_clone as syn::Item);

	// Parse as a list of expressions separated by commas
	let args = parse_macro_input!(args with Punctuated::<Expr, Token![,]>::parse_terminated);
	let args_vec: Vec<_> = args.into_iter().collect();

	// Parse into key-value map
	let mut kwargs = std::collections::HashMap::new();

	// Parse the first argument as the source type
	if let Some(first_arg) = args_vec.first() {
		if let Expr::Path(path_expr) = first_arg {
			if path_expr.path.segments.len() == 1 {
				let source_type = path_expr.path.segments[0].ident.to_string();
				kwargs.insert("src".to_string(), source_type);
			}
		}
	}

	// If source is "above", parse the doc comment and replace it
	if kwargs.get("src").map(|s| s.as_str()) == Some("above") {
		match parse_above_into_kwargs(&mut item) {
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
		parse_cite_kwargs(&args_vec[1..], &mut kwargs);
	}

	// Validate and create citation
	let citation = match validate_with_kwargs(&kwargs) {
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
	// For kwargs syntax, store the parsed kwargs
	kwargs: Option<std::collections::HashMap<String, String>>,
}

/// Handle citation on a function
fn handle_function_citation(citation: Citation, mut item_fn: ItemFn) -> proc_macro2::TokenStream {
	// Generate validation code that runs at compile time
	let (link_text, warning_text, validation_code) = generate_validation_code(&citation);

	// Insert the validation as a const block at the beginning of the function
	let validation_stmt: syn::Stmt = parse_quote! {
		const _: () = { #validation_code };
	};

	item_fn.block.stmts.insert(0, validation_stmt);

	// Add citation footnote to doc comments
	add_citation_footnote_to_item(&mut item_fn.attrs, &citation, link_text, warning_text);

	quote! { #item_fn }
}

/// Handle citation on a struct
fn handle_struct_citation(
	citation: Citation,
	mut item_struct: ItemStruct,
) -> proc_macro2::TokenStream {
	let (link_text, warning_text, validation_code) = generate_validation_code(&citation);
	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	// Add citation footnote to doc comments
	add_citation_footnote_to_item(&mut item_struct.attrs, &citation, link_text, warning_text);

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
	let (link_text, warning_text, validation_code) = generate_validation_code(&citation);
	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	// Add citation footnote to doc comments
	add_citation_footnote_to_item(&mut item_trait.attrs, &citation, link_text, warning_text);

	quote! {
		#item_trait

		const #validation_const_name: () = { #validation_code };
	}
}

/// Handle citation on an impl block
fn handle_impl_citation(citation: Citation, mut item_impl: ItemImpl) -> proc_macro2::TokenStream {
	let (link_text, warning_text, validation_code) = generate_validation_code(&citation);

	// Use counter for unique const name
	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	// Add citation footnote to doc comments
	add_citation_footnote_to_item(&mut item_impl.attrs, &citation, link_text, warning_text);

	quote! {
		#item_impl

		const #validation_const_name: () = { #validation_code };
	}
}

/// Handle citation on a module
fn handle_mod_citation(citation: Citation, mut item_mod: ItemMod) -> proc_macro2::TokenStream {
	let (link_text, warning_text, validation_code) = generate_validation_code(&citation);
	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	// Add citation footnote to doc comments
	add_citation_footnote_to_item(&mut item_mod.attrs, &citation, link_text, warning_text);

	quote! {
		#item_mod

		const #validation_const_name: () = { #validation_code };
	}
}

/// Handle citation on an enum
fn handle_enum_citation(citation: Citation, mut item_enum: ItemEnum) -> proc_macro2::TokenStream {
	let (link_text, warning_text, validation_code) = generate_validation_code(&citation);

	let validation_const_name = syn::Ident::new(
		&format!("_CITE_VALIDATION_{}", next_validation_id()),
		proc_macro2::Span::call_site(),
	);

	// Add citation footnote to doc comments
	add_citation_footnote_to_item(&mut item_enum.attrs, &citation, link_text, warning_text);

	quote! {
		#item_enum

		const #validation_const_name: () = { #validation_code };
	}
}

/// Add citation footnote to doc comments
fn add_citation_footnote_to_item(
	attrs: &mut Vec<syn::Attribute>,
	citation: &Citation,
	link_text: Option<String>,
	warning_text: String,
) {
	// Check if global formatting watermark already exists
	let has_watermark = has_citation_watermark(attrs);

	// Generate the complete footnote
	let mut complete_footnote = String::new();

	// Add global formatting only if watermark doesn't exist
	if !has_watermark {
		complete_footnote.push_str(&generate_global_citation_formatting());
	}

	// Add the specific citation footnote
	complete_footnote.push_str(&generate_citation_footnote(citation, link_text, warning_text));

	// Create a new doc comment attribute
	let doc_attr = parse_quote! {
		#[doc = #complete_footnote]
	};

	// Add it to the attributes
	attrs.push(doc_attr);
}

/// Check if the global citation formatting watermark already exists in the attributes
fn has_citation_watermark(attrs: &[syn::Attribute]) -> bool {
	for attr in attrs {
		if let syn::Meta::NameValue(name_value) = &attr.meta {
			if name_value.path.is_ident("doc") {
				if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit_str), .. }) =
					&name_value.value
				{
					let doc_content = lit_str.value();
					// Check for the citation watermark
					if doc_content.contains(
						"Cited with <a href=\"https://github.com/ramate-io/cite\">cite</a>",
					) {
						return true;
					}
				}
			}
		}
	}
	false
}

/// Generate the global citation formatting (badge and behavior hint)
fn generate_global_citation_formatting() -> String {
	let mut global_formatting = String::new();

	// Add citation badge linking to the cite repo
	global_formatting.push_str("## References\n\n");
	global_formatting.push_str(
		"\n\n<div style=\"background-color:#E6E6FA; border-left:4px solid #9370DB; padding:8px; font-weight:bold;\">\
	Cited with <a href=\"https://github.com/ramate-io/cite\">cite</a>.\
	</div>\n\n"
	);

	// Add behavior hint boxes
	let behavior = cite_core::CitationBehavior::from_features();

	// Global behavior
	match behavior.global {
		cite_core::CitationGlobal::Strict => {
			global_formatting.push_str(
				"\n\n<div style=\"background-color:#F0FFF0; border-left:4px solid #28A745; padding:8px;\">\
	This code uses strict citation validation.\
	</div>\n\n",
			);
		}
		cite_core::CitationGlobal::Lenient => {
			global_formatting.push_str(
				"\n\n<div style=\"background-color:#FFFBE6; border-left:4px solid #FFC107; padding:8px;\">\
	This code uses lenient citation validation.\
	</div>\n\n",
			);
		}
	}

	// Annotation behavior
	match behavior.annotation {
		cite_core::CitationAnnotation::Footnote => {
			global_formatting.push_str(
				"\n\n<div style=\"background-color:#F0FFF0; border-left:4px solid #28A745; padding:8px;\">\
	Annotations are required for citations.\
	</div>\n\n",
			);
		}
		cite_core::CitationAnnotation::Any => {
			global_formatting.push_str(
				"\n\n<div style=\"background-color:#FFFBE6; border-left:4px solid #FFC107; padding:8px;\">\
	Annotations are optional for citations.\
	</div>\n\n",
			);
		}
	}

	// Level behavior
	match behavior.level {
		cite_core::CitationLevel::Error => {
			global_formatting.push_str(
				"\n\n<div style=\"background-color:#F0FFF0; border-left:4px solid #28A745; padding:8px;\">\
	Citation validation errors will fail compilation.\
	</div>\n\n",
			);
		}
		cite_core::CitationLevel::Warn => {
			global_formatting.push_str(
				"\n\n<div style=\"background-color:#FFFBE6; border-left:4px solid #FFC107; padding:8px;\">\
	Citation validation issues will generate warnings.\
	</div>\n\n",
			);
		}
		cite_core::CitationLevel::Silent => {
			global_formatting.push_str(
				"\n\n<div style=\"background-color:#FFEBEE; border-left:4px solid #F44336; padding:8px;\">\
	Citation validation issues will be silently ignored.\
	</div>\n\n",
			);
		}
	}

	global_formatting
}

/// Generate citation footnote text
fn generate_citation_footnote(
	citation: &Citation,
	link_text: Option<String>,
	warning_text: String,
) -> String {
	let mut footnote = String::new();

	// Use provided link text or generate fallback
	let source_ref =
		if let Some(link) = link_text { link } else { generate_source_reference(citation) };

	// Add annotation and level modifiers
	let mut modifiers = Vec::new();
	if let Some(level) = &citation.level {
		modifiers.push(format!("level={}", level.to_uppercase()));
	}
	if let Some(annotation) = &citation.annotation {
		modifiers.push(format!("annotation={}", annotation.to_uppercase()));
	}

	// Build the enumerated footnote
	footnote.push_str("\n1. ");
	footnote.push_str(&source_ref);
	if !modifiers.is_empty() {
		footnote.push_str(&format!(" [{}]", modifiers.join(", ")));
	}

	// Add reason if provided
	if let Some(reason) = &citation.reason {
		// Handle multiline reasons by splitting and prefixing each line with tab
		let formatted_reason =
			reason.lines().map(|line| format!("\t{}", line)).collect::<Vec<_>>().join("\n");

		footnote.push_str(&format!("\n\n{}", formatted_reason));
	}

	if !warning_text.is_empty() {
		// Handle multiline warning text by splitting and prefixing each line with tab
		let formatted_warning = warning_text
			.lines()
			.map(|line| format!("\t>{}", line))
			.collect::<Vec<_>>()
			.join("\n");

		// warning box for warning text
		footnote.push_str(&format!("\n\n\t**Warning!**\n\n{}", formatted_warning));
	}

	footnote
}

/// Generate source reference with hyperlink where applicable
fn generate_source_reference(citation: &Citation) -> String {
	// Try to extract source information from the citation
	if let Some(args) = &citation.raw_args {
		if !args.is_empty() {
			// Check if this is a git source
			if let Some(first_arg) = args.first() {
				if let syn::Expr::Path(path_expr) = first_arg {
					if path_expr.path.segments.len() == 1
						&& path_expr.path.segments[0].ident == "git"
					{
						// Try to construct the GitSource and use its link method
						if let Some(git_source) =
							git::try_construct_git_source_from_citation_args(args)
						{
							return generate_git_source_reference(&git_source);
						}
						// Fallback to manual parsing if construction fails
						return generate_git_source_reference_from_args_fallback(args);
					} else if path_expr.path.segments.len() == 1
						&& path_expr.path.segments[0].ident == "http"
					{
						return generate_http_source_reference(args);
					} else if path_expr.path.segments.len() == 1
						&& path_expr.path.segments[0].ident == "mock"
					{
						return generate_mock_source_reference(args);
					}
				}
			}
		}
	}

	// Fallback for unknown source types
	"Unknown source".to_string()
}

/// Generate git source reference with hyperlink from macro arguments (fallback)
fn generate_git_source_reference_from_args_fallback(args: &[syn::Expr]) -> String {
	let mut remote = None;
	let mut path = None;
	let mut referenced_revision = None;
	let mut name = None;

	// Extract git source parameters
	for arg in args {
		if let syn::Expr::Assign(assign_expr) = arg {
			if let syn::Expr::Path(left_path) = &*assign_expr.left {
				if left_path.path.segments.len() == 1 {
					let param_name = &left_path.path.segments[0].ident.to_string();

					match param_name.as_str() {
						"remote" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								remote = Some(lit_str.value());
							}
						}
						"path" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								path = Some(lit_str.value());
							}
						}
						"referenced_revision" | "ref_rev" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								referenced_revision = Some(lit_str.value());
							}
						}
						"name" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								name = Some(lit_str.value());
							}
						}
						_ => {}
					}
				}
			}
		}
	}

	// Build the reference
	if let (Some(remote_url), Some(file_path), Some(rev)) = (remote, path, referenced_revision) {
		// Use the provided name as link text if available, otherwise use default format
		let link_text = if let Some(custom_name) = name {
			custom_name
		} else {
			let short_rev = if rev.len() > 8 { &rev[..8] } else { &rev };
			format!("Git: {} @ {}", file_path, short_rev)
		};

		// Try to create a hyperlink for GitHub URLs
		if remote_url.contains("github.com") {
			// Extract owner/repo from GitHub URL
			if let Some(_repo_part) = remote_url.split("github.com/").nth(1) {
				return format!(
					"[{}]({}/blob/{}/{}#L1)",
					link_text,
					remote_url.trim_end_matches(".git"),
					rev,
					file_path
				);
			}
		}

		// Fallback for non-GitHub URLs
		format!("{} ({})", link_text, remote_url)
	} else {
		"Git source (incomplete parameters)".to_string()
	}
}

/// Generate git source reference with hyperlink using GitSource
fn generate_git_source_reference(git_source: &cite_git::GitSource) -> String {
	use cite_core::Source;

	let name = git_source.name();
	let url = git_source.link();

	// Format as [name](url)
	format!("**[{}]({})**", name, url)
}

/// Generate HTTP source reference with hyperlink from source object
fn generate_http_source_reference_from_source(http_source: &cite_http::HttpMatch) -> String {
	use cite_core::Source;

	let url = http_source.source_url.as_str();
	let link_text = http_source.link();

	format!("**[{}]({})**", link_text, url)
}

/// Generate HTTP source reference with hyperlink from macro arguments
fn generate_http_source_reference(args: &[syn::Expr]) -> String {
	let mut url = None;
	let mut pattern = None;
	let mut selector = None;

	// Extract HTTP source parameters
	for arg in args {
		if let syn::Expr::Assign(assign_expr) = arg {
			if let syn::Expr::Path(left_path) = &*assign_expr.left {
				if left_path.path.segments.len() == 1 {
					let name = &left_path.path.segments[0].ident.to_string();

					match name.as_str() {
						"url" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								url = Some(lit_str.value());
							}
						}
						"pattern" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								pattern = Some(lit_str.value());
							}
						}
						"selector" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								selector = Some(lit_str.value());
							}
						}
						_ => {}
					}
				}
			}
		}
	}

	// Build the reference
	if let Some(url_str) = url {
		let extraction_method = if pattern.is_some() {
			"regex pattern"
		} else if selector.is_some() {
			"CSS selector"
		} else {
			"full content"
		};

		return format!("[HTTP: {}]({})", extraction_method, url_str);
	} else {
		"HTTP source (incomplete parameters)".to_string()
	}
}

/// Generate mock source reference from source object
fn generate_mock_source_reference_from_source(mock_source: &cite_core::mock::MockSource) -> String {
	use cite_core::Source;

	let link_text = mock_source.link();
	format!("[{}](https://github.com/ramate-io/cite#mock-sources)", link_text)
}

/// Generate mock source reference from macro arguments
fn generate_mock_source_reference(args: &[syn::Expr]) -> String {
	let mut same = None;
	let mut changed = None;

	// Extract mock source parameters
	for arg in args {
		if let syn::Expr::Assign(assign_expr) = arg {
			if let syn::Expr::Path(left_path) = &*assign_expr.left {
				if left_path.path.segments.len() == 1 {
					let name = &left_path.path.segments[0].ident.to_string();

					match name.as_str() {
						"same" => {
							if let syn::Expr::Lit(syn::ExprLit {
								lit: syn::Lit::Str(lit_str),
								..
							}) = &*assign_expr.right
							{
								same = Some(lit_str.value());
							}
						}
						"changed" => {
							// Handle tuple syntax for changed
							if let syn::Expr::Tuple(tuple_expr) = &*assign_expr.right {
								if tuple_expr.elems.len() == 2 {
									if let (Some(old), Some(new)) =
										(tuple_expr.elems.first(), tuple_expr.elems.get(1))
									{
										if let (
											syn::Expr::Lit(syn::ExprLit {
												lit: syn::Lit::Str(old_lit),
												..
											}),
											syn::Expr::Lit(syn::ExprLit {
												lit: syn::Lit::Str(new_lit),
												..
											}),
										) = (old, new)
										{
											changed = Some((old_lit.value(), new_lit.value()));
										}
									}
								}
							}
						}
						_ => {}
					}
				}
			}
		}
	}

	// Build the reference
	if let Some(content) = same {
		let preview = if content.len() > 50 { format!("{}...", &content[..50]) } else { content };
		format!("[Mock: same = \"{}\"](https://github.com/ramate-io/cite#mock-sources)", preview)
	} else if let Some((old, new)) = changed {
		let old_preview = if old.len() > 30 { format!("{}...", &old[..30]) } else { old };
		let new_preview = if new.len() > 30 { format!("{}...", &new[..30]) } else { new };
		format!(
			"[Mock: changed = (\"{}\", \"{}\")](https://github.com/ramate-io/cite#mock-sources)",
			old_preview, new_preview
		)
	} else {
		"[Mock source (incomplete parameters)](https://github.com/ramate-io/cite#mock-sources)"
			.to_string()
	}
}

/// Construct a source from citation arguments and return its link text
fn construct_source_from_citation(citation: &Citation) -> Option<String> {
	if let Some(args) = &citation.raw_args {
		if !args.is_empty() {
			// Try Git sources first (since git is the most common)
			if let Some(git_source) = git::try_construct_git_source_from_citation_args(args) {
				return Some(generate_git_source_reference(&git_source));
			}

			// Try HTTP sources
			if let Some(http_source) = http::try_construct_http_source_from_citation_args(args) {
				return Some(generate_http_source_reference_from_source(&http_source));
			}

			// Try mock sources
			if let Some(mock_source) = mock::try_construct_mock_source_from_citation_args(args) {
				return Some(generate_mock_source_reference_from_source(&mock_source));
			}
		}
	}

	// Try to construct and execute MockSource using the traditional expression parsing
	if let Some(mock_source) = mock::try_construct_mock_source_from_expr(&citation.source_expr) {
		return Some(generate_mock_source_reference_from_source(&mock_source));
	}

	None
}

/// Generate validation code that executes the user's source expression with the real API
/// Returns (link_text, warning_text, validation_code)
fn generate_validation_code(
	citation: &Citation,
) -> (Option<String>, String, proc_macro2::TokenStream) {
	// Try to construct the source first
	let link_text = construct_source_from_citation(citation);
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

	(link_text, warning_text, validation_code)
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
	// Check if this uses kwargs syntax by looking for the unit expression
	if let Expr::Tuple(tuple_expr) = &citation.source_expr {
		if tuple_expr.elems.is_empty() {
			// This is a unit expression, check if we have kwargs
			if citation.kwargs.is_some() {
				return execute_kwargs_source_validation(citation, behavior, level_override);
			}
		}
	}

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

	// Check if this uses the new syntax where the source type is the first argument
	if let Some(args) = &citation.raw_args {
		if !args.is_empty() {
			// Try Git sources first (since git is the most common)
			if let Some(git_source) = git::try_construct_git_source_from_citation_args(args) {
				return execute_git_source_validation(git_source, behavior, level_override);
			}

			// Try HTTP sources
			if let Some(http_source) = http::try_construct_http_source_from_citation_args(args) {
				return execute_http_source_validation(http_source, behavior, level_override);
			}

			// Try mock sources
			if let Some(mock_source) = mock::try_construct_mock_source_from_citation_args(args) {
				return execute_mock_source_validation(mock_source, behavior, level_override);
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

/// Execute kwargs source validation and return the result
fn execute_kwargs_source_validation(
	_citation: &Citation,
	_behavior: &cite_core::CitationBehavior,
	_level_override: Option<cite_core::CitationLevel>,
) -> Option<std::result::Result<Option<String>, String>> {
	// The kwargs have already been validated in validate_with_kwargs
	// Just return success
	Some(Ok(None))
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
	use cite_core::Source;

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

/// Parse doc comment into key-value map and remove the cite above content
fn parse_above_into_kwargs(
	item: &mut syn::Item,
) -> Result<std::collections::HashMap<String, String>> {
	// Look for doc comments in the item's attributes
	let attrs = match item {
		syn::Item::Fn(item_fn) => &mut item_fn.attrs,
		syn::Item::Struct(item_struct) => &mut item_struct.attrs,
		syn::Item::Enum(item_enum) => &mut item_enum.attrs,
		syn::Item::Trait(item_trait) => &mut item_trait.attrs,
		syn::Item::Impl(item_impl) => &mut item_impl.attrs,
		syn::Item::Mod(item_mod) => &mut item_mod.attrs,
		_ => return Err(syn::Error::new(
			proc_macro2::Span::call_site(),
			"cite above can only be used on functions, structs, enums, traits, impl blocks, or modules",
		)),
	};

	// Find the doc attribute that contains <cite above> and extract content
	for attr in attrs {
		if attr.path().is_ident("doc") {
			if let syn::Meta::NameValue(meta_name_value) = &attr.meta {
				if let syn::Expr::Lit(expr_lit) = &meta_name_value.value {
					if let syn::Lit::Str(lit_str) = &expr_lit.lit {
						let doc_content = lit_str.value();
						if doc_content.contains("<cite above>")
							&& doc_content.contains("</cite above>")
						{
							// Extract the content between the tags
							if let Some(cite_content) = extract_cite_content(&doc_content) {
								// Remove the <cite above> content from the doc comment
								remove_cite_above_from_doc_comment(attr, &doc_content);

								// Parse the JSON content
								return parse_json_content_to_kwargs(&cite_content);
							}
						}
					}
				}
			}
		}
	}

	Err(syn::Error::new(
		proc_macro2::Span::call_site(),
		"no <cite above> block found in doc comments",
	))
}

/// Extract content between <cite above> and </cite above> tags and remove the tags and content
fn extract_cite_content(doc_content: &str) -> Option<String> {
	let start_tag = "<cite above>";
	let end_tag = "</cite above>";

	if let Some(start_pos) = doc_content.find(start_tag) {
		if let Some(end_pos) = doc_content.find(end_tag) {
			let start = start_pos + start_tag.len();
			if start < end_pos {
				return Some(doc_content[start..end_pos].trim().to_string());
			}
		}
	}
	None
}

/// Remove the <cite above> content from a doc comment while keeping the rest
fn remove_cite_above_from_doc_comment(attr: &mut syn::Attribute, doc_content: &str) {
	let start_tag = "<cite above>";
	let end_tag = "</cite above>";

	println!("doc_content: {}", doc_content);

	if let Some(start_pos) = doc_content.find(start_tag) {
		if let Some(end_pos) = doc_content.find(end_tag) {
			// Create new doc content without the <cite above> section
			let before_cite = doc_content[..start_pos].trim();
			let after_cite = doc_content[end_pos + end_tag.len()..].trim();

			let new_doc_content = if before_cite.is_empty() && after_cite.is_empty() {
				"".to_string()
			} else if before_cite.is_empty() {
				after_cite.to_string()
			} else if after_cite.is_empty() {
				before_cite.to_string()
			} else {
				format!("{}\n\n{}", before_cite, after_cite)
			};

			println!("new_doc_content: {}", new_doc_content);

			// Update the attribute with the new content
			if let syn::Meta::NameValue(meta_name_value) = &mut attr.meta {
				if let syn::Expr::Lit(expr_lit) = &mut meta_name_value.value {
					if let syn::Lit::Str(_) = &mut expr_lit.lit {
						// Create a new LitStr with the updated content
						println!("new_doc_content: {}", new_doc_content);
						expr_lit.lit = syn::Lit::Str(syn::LitStr::new(
							&new_doc_content,
							proc_macro2::Span::call_site(),
						));
					}
				}
			}
		}
	}
}

/// Parse JSON content into key-value map
fn parse_json_content_to_kwargs(
	cite_content: &str,
) -> Result<std::collections::HashMap<String, String>> {
	// Parse the JSON content
	let json_value: serde_json::Value = serde_json::from_str(cite_content).map_err(|e| {
		syn::Error::new(
			proc_macro2::Span::call_site(),
			format!("invalid JSON in <cite above> block: {}", e),
		)
	})?;

	// Convert JSON object to HashMap<String, String>
	let mut kwargs = std::collections::HashMap::new();

	if let serde_json::Value::Object(obj) = json_value {
		for (key, value) in obj {
			if let serde_json::Value::String(s) = value {
				kwargs.insert(key, s);
			} else {
				return Err(syn::Error::new(
					proc_macro2::Span::call_site(),
					format!("all values in <cite above> JSON must be strings, found: {:?}", value),
				));
			}
		}
	} else {
		return Err(syn::Error::new(
			proc_macro2::Span::call_site(),
			"<cite above> JSON must be an object",
		));
	}

	// Ensure src field is present
	if !kwargs.contains_key("src") {
		return Err(syn::Error::new(
			proc_macro2::Span::call_site(),
			"<cite above> JSON must contain a 'src' field",
		));
	}

	Ok(kwargs)
}

/// Parse cite arguments into key-value map
fn parse_cite_kwargs(args: &[Expr], kwargs: &mut std::collections::HashMap<String, String>) {
	for arg in args {
		if let Expr::Assign(assign_expr) = arg {
			if let Expr::Path(left_path) = &*assign_expr.left {
				if left_path.path.segments.len() == 1 {
					let key = left_path.path.segments[0].ident.to_string();
					if let Expr::Lit(expr_lit) = &*assign_expr.right {
						if let Lit::Str(lit_str) = &expr_lit.lit {
							kwargs.insert(key, lit_str.value());
						}
					}
				}
			}
		}
	}
}

/// Validate kwargs and create citation
fn validate_with_kwargs(kwargs: &std::collections::HashMap<String, String>) -> Result<Citation> {
	let src = kwargs.get("src").ok_or_else(|| {
		syn::Error::new(proc_macro2::Span::call_site(), "cite attribute requires a source type")
	})?;

	let reason = kwargs.get("reason").cloned();
	let level = kwargs.get("level").cloned();
	let annotation = kwargs.get("annotation").cloned();

	// Validate source-specific parameters
	match src.as_str() {
		"git" => {
			// Validate git source parameters
			kwargs.get("remote").ok_or_else(|| {
				syn::Error::new(
					proc_macro2::Span::call_site(),
					"git source requires 'remote = \"...\"'",
				)
			})?;
			kwargs.get("ref_rev").ok_or_else(|| {
				syn::Error::new(
					proc_macro2::Span::call_site(),
					"git source requires 'ref_rev = \"...\"'",
				)
			})?;
			kwargs.get("cur_rev").ok_or_else(|| {
				syn::Error::new(
					proc_macro2::Span::call_site(),
					"git source requires 'cur_rev = \"...\"'",
				)
			})?;
			kwargs.get("path").ok_or_else(|| {
				syn::Error::new(
					proc_macro2::Span::call_site(),
					"git source requires 'path = \"...\"'",
				)
			})?;
		}
		"http" => {
			// Validate http source parameters
			kwargs.get("url").ok_or_else(|| {
				syn::Error::new(
					proc_macro2::Span::call_site(),
					"http source requires 'url = \"...\"'",
				)
			})?;
		}
		"mock" => {
			// Validate mock source parameters
			let same = kwargs.get("same");
			let changed = kwargs.get("changed");
			if same.is_none() && changed.is_none() {
				return Err(syn::Error::new(
					proc_macro2::Span::call_site(),
					"mock source requires 'same = \"...\"' or 'changed = (\"old\", \"new\")'",
				));
			}
		}
		_ => {
			return Err(syn::Error::new(
				proc_macro2::Span::call_site(),
				format!("unknown source type: {}", src),
			));
		}
	}

	// Create a simple source expression - just a unit tuple
	let source_expr = syn::parse_quote! { () };

	Ok(Citation {
		source_expr,
		reason,
		level,
		annotation,
		raw_args: None,
		kwargs: Some(kwargs.clone()),
	})
}
