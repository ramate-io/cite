/// Macro should expand to a GitSource declaration of the following form:
/// ```rust,ignore
/// helper_macro_git!(
///     doc = 2,
/// );
///
/// #[cite(
///     git,
///     remote = "https://github.com/ramate-io/cite",
///     ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
///     cur_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
///     path = "tests/helper-macro-git/helper-macro-git/DOC_2.md",
/// )]
/// pub fn test_git_source() {
///     println!("This function has a citation with a git source");
/// }
/// ```
///
/// Optionally, you can override the ref_rev, cur_rev, and reason rev outside the macro:
/// ```rust,ignore
/// #[cite(
///     helper_macro_git(doc = 2),
///     ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
///     cur_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
///     reason = "Testing git source"
/// )]
/// pub fn test_git_source() {
///     println!("This function has a citation with a git source");
/// }
/// ```
use proc_macro::TokenStream;
use syn::{parse_macro_input, Expr, Lit, Result};

#[proc_macro_attribute]
pub fn helper_macro_git(args: TokenStream, input: TokenStream) -> TokenStream {
	// Parse the arguments to extract the doc number
	let args = parse_macro_input!(args with syn::punctuated::Punctuated<Expr, syn::Token![,]>::parse_terminated);

	// Extract the doc number from the arguments
	let doc_num = match extract_doc_number(&args) {
		Ok(num) => num,
		Err(err) => return err.to_compile_error().into(),
	};

	// Parse the input item
	let mut item = parse_macro_input!(input as syn::Item);

	// Add the doc comment with the cite above content as JSON
	let json_data = serde_json::json!({
		"src": "git",
		"remote": "https://github.com/ramate-io/cite",
		"ref_rev": "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
		"cur_rev": "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
		"path": format!("tests/helper-macro-git/helper-macro-git/DOC_{}.md", doc_num)
	});

	let doc_comment = format!(
		"<cite above>\n{}\n</cite above>",
		serde_json::to_string_pretty(&json_data).unwrap()
	);

	// Add the doc attribute to the item
	add_doc_attribute(&mut item, &doc_comment);

	// Return the modified item
	quote::quote!(#item).into()
}

/// Extract the doc number from the macro arguments
fn extract_doc_number(args: &syn::punctuated::Punctuated<Expr, syn::Token![,]>) -> Result<u32> {
	if args.len() != 1 {
		return Err(syn::Error::new(
			proc_macro2::Span::call_site(),
			"helper_macro_git expects exactly one argument: doc = <number>",
		));
	}

	let arg = &args[0];
	if let Expr::Assign(assign_expr) = arg {
		if let Expr::Path(left_path) = &*assign_expr.left {
			if left_path.path.segments.len() == 1 && left_path.path.segments[0].ident == "doc" {
				if let Expr::Lit(expr_lit) = &*assign_expr.right {
					if let Lit::Int(lit_int) = &expr_lit.lit {
						return lit_int.base10_parse::<u32>();
					}
				}
			}
		}
	}

	Err(syn::Error::new(proc_macro2::Span::call_site(), "helper_macro_git expects: doc = <number>"))
}

/// Add a doc attribute to the item
fn add_doc_attribute(item: &mut syn::Item, doc_content: &str) {
	let doc_attr = syn::parse_quote! {
		#[doc = #doc_content]
	};

	match item {
		syn::Item::Fn(item_fn) => {
			item_fn.attrs.insert(0, doc_attr);
		}
		syn::Item::Struct(item_struct) => {
			item_struct.attrs.insert(0, doc_attr);
		}
		syn::Item::Enum(item_enum) => {
			item_enum.attrs.insert(0, doc_attr);
		}
		syn::Item::Trait(item_trait) => {
			item_trait.attrs.insert(0, doc_attr);
		}
		syn::Item::Impl(item_impl) => {
			item_impl.attrs.insert(0, doc_attr);
		}
		syn::Item::Mod(item_mod) => {
			item_mod.attrs.insert(0, doc_attr);
		}
		_ => {
			// For other item types, we'll just ignore them
		}
	}
}
