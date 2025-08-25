//! Mock source parsing for procedural macros
//! 
//! This module handles parsing of mock source expressions in the #[cite] attribute.

pub mod macro_syntax;
pub mod struct_syntax;

pub use macro_syntax::*;
pub use struct_syntax::*;