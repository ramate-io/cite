use quote::ToTokens;
use syn::{Expr, Lit};

pub mod above;

/// Parse cite arguments into key-value map
pub fn parse_cite_kwargs(args: &[Expr]) -> std::collections::HashMap<String, serde_json::Value> {
	let mut kwargs = std::collections::HashMap::new();

	for arg in args {
		if let Expr::Assign(assign_expr) = arg {
			if let Expr::Path(left_path) = &*assign_expr.left {
				if left_path.path.segments.len() == 1 {
					let key = left_path.path.segments[0].ident.to_string();

					// Parse the value based on its type
					let value = match &*assign_expr.right {
						Expr::Lit(expr_lit) => match &expr_lit.lit {
							Lit::Str(lit_str) => serde_json::Value::String(lit_str.value()),
							Lit::Int(lit_int) => {
								if let Ok(int_val) = lit_int.base10_parse::<i64>() {
									serde_json::Value::Number(serde_json::Number::from(int_val))
								} else {
									serde_json::Value::String(lit_int.to_token_stream().to_string())
								}
							}
							Lit::Float(lit_float) => {
								if let Ok(float_val) = lit_float.base10_parse::<f64>() {
									serde_json::Value::Number(
										serde_json::Number::from_f64(float_val)
											.unwrap_or_else(|| serde_json::Number::from(0)),
									)
								} else {
									serde_json::Value::String(
										lit_float.to_token_stream().to_string(),
									)
								}
							}
							Lit::Bool(lit_bool) => serde_json::Value::Bool(lit_bool.value),
							_ => serde_json::Value::String(
								assign_expr.right.to_token_stream().to_string(),
							),
						},
						Expr::Tuple(tuple_expr) => {
							// Handle tuple expressions like ("a", "b")
							let mut tuple_values = Vec::new();
							for elem in &tuple_expr.elems {
								if let Expr::Lit(expr_lit) = elem {
									match &expr_lit.lit {
										Lit::Str(lit_str) => tuple_values
											.push(serde_json::Value::String(lit_str.value())),
										Lit::Int(lit_int) => {
											if let Ok(int_val) = lit_int.base10_parse::<i64>() {
												tuple_values.push(serde_json::Value::Number(
													serde_json::Number::from(int_val),
												));
											} else {
												tuple_values.push(serde_json::Value::String(
													lit_int.to_token_stream().to_string(),
												));
											}
										}
										_ => tuple_values.push(serde_json::Value::String(
											elem.to_token_stream().to_string(),
										)),
									}
								} else {
									tuple_values.push(serde_json::Value::String(
										elem.to_token_stream().to_string(),
									));
								}
							}
							serde_json::Value::Array(tuple_values)
						}
						_ => serde_json::Value::String(
							assign_expr.right.to_token_stream().to_string(),
						),
					};

					kwargs.insert(key, value);
				}
			}
		}
	}

	kwargs
}

/// Extract the first argument as the source type
pub fn extract_source_type(args: &[Expr]) -> Option<String> {
	if let Some(first_arg) = args.first() {
		if let Expr::Path(path_expr) = first_arg {
			if path_expr.path.segments.len() == 1 {
				return Some(path_expr.path.segments[0].ident.to_string());
			}
		}
	}
	None
}
