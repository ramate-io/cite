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
#[macro_export]
macro_rules! helper_macro_git {
    (doc = $doc_num:expr) => {
        git,
        remote = "https://github.com/ramate-io/cite",
        ref_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
        cur_rev = "94dab273cf6c2abe8742d6d459ad45c96ca9b694",
        path = concat!("tests/helper-macro-git/helper-macro-git/DOC_", stringify!($doc_num), ".md")
    };
}
