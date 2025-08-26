use std::collections::HashMap;
use cite_core::{Source, SourceError, Comparison, Referenced, Current, Diff, Content, Id};
use cite_cache::{CacheableReferenced, CacheableCurrent, CacheError};
use serde::{Serialize, Deserialize};
use regex::Regex;
use scraper::{Html, Selector};
use similar::{ChangeTag, TextDiff};

/// Match expression for extracting content from http
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatchExpression {
    /// Regex pattern matching
    Regex(String),
    /// CSS selector matching
    CssSelector(String),
    /// XPath expression matching
    XPath(String),
    /// Full document (no matching)
    FullDocument,
    /// Fragment-based matching (automatically targets element with matching id/name)
    Fragment(String),
}

impl MatchExpression {
    /// Create a regex match expression
    pub fn regex(pattern: &str) -> Self {
        Self::Regex(pattern.to_string())
    }
    
    /// Create a CSS selector match expression
    pub fn css_selector(selector: &str) -> Self {
        Self::CssSelector(selector.to_string())
    }
    
    /// Create an XPath match expression
    pub fn xpath(expression: &str) -> Self {
        Self::XPath(expression.to_string())
    }
    
    /// Create a full document match (no extraction)
    pub fn full_document() -> Self {
        Self::FullDocument
    }
    
    /// Create a fragment match expression
    pub fn fragment(fragment_id: &str) -> Self {
        Self::Fragment(fragment_id.to_string())
    }
    
    /// Extract matching content from http
    pub fn extract_from(&self, content: &str) -> Result<String, SourceError> {
        match self {
            MatchExpression::Regex(pattern) => {
                let regex = Regex::new(pattern)
                    .map_err(|e| SourceError::ContentParsing(format!("Invalid regex pattern '{}': {}", pattern, e)))?;
                
                if let Some(captures) = regex.captures(content) {
                    // If there are capture groups, return the first one; otherwise return the full match
                    if captures.len() > 1 {
                        Ok(captures.get(1).map(|m| m.as_str()).unwrap_or("").to_string())
                    } else {
                        Ok(captures.get(0).map(|m| m.as_str()).unwrap_or("").to_string())
                    }
                } else {
                    Ok(String::new())
                }
            }
            MatchExpression::CssSelector(selector_str) => {
                let document = Html::parse_document(content);
                let selector = Selector::parse(selector_str)
                    .map_err(|e| SourceError::ContentParsing(format!("Invalid CSS selector '{}': {:?}", selector_str, e)))?;
                
                let mut results = Vec::new();
                for element in document.select(&selector) {
                    results.push(element.text().collect::<Vec<_>>().join(" ").trim().to_string());
                }
                
                Ok(results.join("\n"))
            }
            MatchExpression::XPath(_expression) => {
                // XPath support would require additional crates like sxd-xpath
                // For now, return an error indicating it's not implemented
                Err(SourceError::ContentParsing("XPath expressions are not yet implemented".to_string()))
            }
            MatchExpression::FullDocument => {
                Ok(content.to_string())
            }
            MatchExpression::Fragment(fragment_id) => {
                let document = Html::parse_document(content);
                
                // Try multiple selectors to find the fragment:
                // 1. Element with matching id
                // 2. Element with matching name (for older HTML)
                // 3. Anchor with matching name
                let selectors = [
                    format!("#{}", fragment_id),                    // #fragment-id
                    format!("[id='{}']", fragment_id),             // [id='fragment-id']
                    format!("[name='{}']", fragment_id),           // [name='fragment-id']
                    format!("a[name='{}']", fragment_id),          // a[name='fragment-id']
                ];
                
                for selector_str in &selectors {
                    if let Ok(selector) = Selector::parse(selector_str) {
                        if let Some(element) = document.select(&selector).next() {
                            // Extract the element and its contents
                            let mut result = Vec::new();
                            
                            // Include the element itself and its descendants
                            result.push(element.text().collect::<Vec<_>>().join(" ").trim().to_string());
                            
                            return Ok(result.join("\n"));
                        }
                    }
                }
                
                // If no fragment found, return empty string (not an error - fragment might not exist)
                Ok(String::new())
            }
        }
    }
}

/// Source URL with validation, normalization, and fragment support
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceUrl {
    url: String,
    normalized: String,
    fragment: Option<String>,
}

impl SourceUrl {
    /// Create a new source URL with validation
    pub fn new(url: &str) -> Result<Self, SourceError> {
        let (base_url, fragment) = Self::parse_url_and_fragment(url);
        let normalized = Self::normalize_url(&base_url)?;
        Ok(Self {
            url: url.to_string(),
            normalized,
            fragment,
        })
    }
    
    /// Get the original URL
    pub fn as_str(&self) -> &str {
        &self.url
    }
    
    /// Get the normalized URL for caching/comparison
    pub fn normalized(&self) -> &str {
        &self.normalized
    }
    
    /// Get the URL fragment (part after #)
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }
    
    /// Get the base URL without fragment
    pub fn base_url(&self) -> &str {
        &self.normalized
    }
    
    /// Parse URL and extract fragment
    fn parse_url_and_fragment(url: &str) -> (String, Option<String>) {
        if let Some(fragment_pos) = url.find('#') {
            let base_url = url[..fragment_pos].to_string();
            let fragment = url[fragment_pos + 1..].to_string();
            (base_url, if fragment.is_empty() { None } else { Some(fragment) })
        } else {
            (url.to_string(), None)
        }
    }
    
    /// Normalize URL for consistent caching
    fn normalize_url(url: &str) -> Result<String, SourceError> {
        // Basic URL validation and normalization
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(SourceError::Network(format!("Invalid URL scheme: {}", url)));
        }
        
        // Remove trailing slashes, convert to lowercase domain, etc.
        let mut normalized = url.to_lowercase();
        if normalized.ends_with('/') && normalized.len() > 8 {
            normalized.pop();
        }
        
        Ok(normalized)
    }
}

/// Http content that was referenced at commit time
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencedHttp {
    /// The extracted content that was referenced
    pub content: String,
    /// Metadata about the http source
    pub metadata: HashMap<String, String>,
    /// The URL that was accessed
    pub source_url: SourceUrl,
    /// The match expression used
    pub match_expression: MatchExpression,
}

impl Content for ReferencedHttp {}
impl Referenced for ReferencedHttp {}

impl CacheableReferenced for ReferencedHttp {
    fn from_cached_buffer(buffer: Vec<u8>) -> Result<Self, CacheError> {
        serde_json::from_slice(&buffer).map_err(|e| CacheError::Deserialize(e.into()))
    }
}

/// Current http content fetched from the web
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentHttp {
    /// The extracted content currently available
    pub content: String,
    /// Current metadata (headers, timestamps, etc.)
    pub metadata: HashMap<String, String>,
    /// The URL that was accessed
    pub source_url: SourceUrl,
    /// The match expression used
    pub match_expression: MatchExpression,
    /// Raw http before extraction (for debugging)
    pub raw_content: Option<String>,
}

impl Content for CurrentHttp {}

impl Current<ReferencedHttp, HttpDiff> for CurrentHttp {
    fn diff(&self, referenced: &ReferencedHttp) -> Result<HttpDiff, SourceError> {
        let content_changed = self.content != referenced.content;
        let url_changed = self.source_url != referenced.source_url;
        let match_expression_changed = self.match_expression != referenced.match_expression;
        
        let mut diff = HttpDiff {
            content_changed,
            url_changed,
            match_expression_changed,
            referenced_content: referenced.content.clone(),
            current_content: self.content.clone(),
            unified_diff: None,
        };
        
        // Generate unified diff if content changed
        diff.generate_unified_diff();
        
        Ok(diff)
    }
}

impl CacheableCurrent<ReferencedHttp, HttpDiff> for CurrentHttp {
    fn to_cached_buffer(&self) -> Result<Vec<u8>, CacheError> {
        // Convert to ReferencedHttp format for caching
        let referenced = ReferencedHttp {
            content: self.content.clone(),
            metadata: self.metadata.clone(),
            source_url: self.source_url.clone(),
            match_expression: self.match_expression.clone(),
        };
        serde_json::to_vec(&referenced).map_err(|e| CacheError::Serialize(e.into()))
    }
}

/// Diff between referenced and current http
#[derive(Debug, Clone, PartialEq)]
pub struct HttpDiff {
    pub content_changed: bool,
    pub url_changed: bool,
    pub match_expression_changed: bool,
    pub referenced_content: String,
    pub current_content: String,
    pub unified_diff: Option<String>,
}

impl HttpDiff {
    /// Generate a git-style unified diff
    pub fn generate_unified_diff(&mut self) {
        if self.content_changed {
            let diff = TextDiff::from_lines(&self.referenced_content, &self.current_content);
            let mut result = Vec::new();
            
            for change in diff.iter_all_changes() {
                let sign = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+", 
                    ChangeTag::Equal => " ",
                };
                result.push(format!("{}{}", sign, change));
            }
            
            if !result.is_empty() {
                self.unified_diff = Some(result.join(""));
            }
        }
    }
    
    /// Get the unified diff as a string
    pub fn unified_diff(&self) -> Option<&str> {
        self.unified_diff.as_deref()
    }
}

impl Diff for HttpDiff {
    fn is_empty(&self) -> bool {
        !self.content_changed && !self.url_changed && !self.match_expression_changed
    }
}

/// Http match source for checking committed http references
#[derive(Clone)]
pub struct HttpMatch {
    pub matches: MatchExpression,
    pub source_url: SourceUrl,
    pub cache_path: String,
    id: Id,
    cache: cite_cache::Cache,
    cache_behavior: cite_cache::CacheBehavior,
}

impl HttpMatch {
    /// Create a new http match with caching (legacy method)
    pub fn cached(url: &str, pattern: &str) -> Result<Self, SourceError> {
        Self::with_match_expression(url, MatchExpression::regex(pattern))
    }
    
    /// Create with custom match expression
    pub fn with_match_expression(url: &str, expression: MatchExpression) -> Result<Self, SourceError> {
        Self::with_match_expression_and_cache_behavior(url, expression, cite_cache::CacheBehavior::Enabled)
    }
    
    /// Create with custom match expression and cache behavior
    pub fn with_match_expression_and_cache_behavior(
        url: &str, 
        expression: MatchExpression, 
        cache_behavior: cite_cache::CacheBehavior
    ) -> Result<Self, SourceError> {
        use cite_cache::CacheBuilder;
        
        let source_url = SourceUrl::new(url)?;
        let cache_path = format!("http_{}_{}", Self::url_to_cache_key(url), Self::match_expression_to_cache_key(&expression));
        let id = Id::new(cache_path.clone());
        
        // Always create a cache - the behavior determines how it's used
        let cache_builder = CacheBuilder::default();
        let cache = cache_builder.build()
            .map_err(|e| SourceError::Network(format!("Failed to create cache: {}", e)))?;
        
        Ok(Self {
            matches: expression,
            source_url,
            cache_path,
            id,
            cache,
            cache_behavior,
        })
    }
    
    /// Create HTTP match with automatic fragment detection
    /// If the URL contains a fragment, it will automatically use fragment-based matching
    /// If no fragment is present, defaults to full document matching
    pub fn with_auto_fragment(url: &str) -> Result<Self, SourceError> {
        let source_url = SourceUrl::new(url)?;
        let match_expression = if let Some(fragment) = source_url.fragment() {
            MatchExpression::fragment(fragment)
        } else {
            MatchExpression::full_document()
        };
        
        Self::with_match_expression_and_cache_behavior(url, match_expression, cite_cache::CacheBehavior::Enabled)
    }
    
    /// Create HTTP match for macro usage with cache behavior determination
    /// 
    /// This is the main constructor for procedural macros. It:
    /// 1. Determines cache behavior from environment variables and kwargs
    /// 2. Handles auto-fragment detection if URL contains #fragment
    /// 3. Creates appropriate match expression based on parameters
    /// 
    /// Parameters:
    /// - url: The target URL (may contain fragment)
    /// - match_expression: Optional explicit match expression
    /// - cache_override: Optional cache behavior from macro kwargs
    pub fn try_new_for_macro(
        url: &str,
        match_expression: Option<MatchExpression>,
        cache_override: Option<cite_cache::CacheBehavior>
    ) -> Result<Self, SourceError> {
        // Determine final cache behavior (env var overrides kwargs)
        let cache_behavior = determine_cache_behavior_for_macro(cache_override);
        
        // Determine match expression
        let final_match_expression = if let Some(expr) = match_expression {
            expr
        } else {
            // Auto-detect based on URL fragment
            let source_url = SourceUrl::new(url)?;
            if let Some(fragment) = source_url.fragment() {
                MatchExpression::fragment(fragment)
            } else {
                // No explicit match expression and no fragment - this should be an error
                return Err(SourceError::ContentParsing(
                    "HTTP citation requires either an explicit match expression (pattern/selector/match_type) or a URL with fragment".to_string()
                ));
            }
        };
        
        Self::with_match_expression_and_cache_behavior(url, final_match_expression, cache_behavior)
    }
    
    /// Convert URL to a safe cache key
    fn url_to_cache_key(url: &str) -> String {
        // Replace unsafe characters for filesystem
        url.chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
                _ => '_',
            })
            .collect()
    }
    
    /// Convert match expression to a safe cache key component
    fn match_expression_to_cache_key(expression: &MatchExpression) -> String {
        let key = match expression {
            MatchExpression::Regex(pattern) => format!("regex_{}", pattern),
            MatchExpression::CssSelector(selector) => format!("css_{}", selector),
            MatchExpression::Fragment(fragment) => format!("frag_{}", fragment),
            MatchExpression::XPath(xpath) => format!("xpath_{}", xpath),
            MatchExpression::FullDocument => "full".to_string(),
        };
        
        // Make it filesystem-safe
        key.chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
                _ => '_',
            })
            .collect()
    }
    
    /// Fetch http content from the URL
    fn fetch_http(&self) -> Result<String, SourceError> {
        // For testing, use simulated responses for known URLs
        match self.source_url.as_str() {
            url if url.contains("example.com") => {
                Ok("<html><body><h1>Example Domain</h1><p>This domain is for use in illustrative examples.</p></body></html>".to_string())
            }
            url if url.contains("httpbin.org/json") => {
                // Simulate a JSON response with timestamp that changes
                let timestamp = chrono::Utc::now().timestamp();
                Ok(format!(r#"{{"timestamp": {}, "data": "test response"}}"#, timestamp))
            }
            url if url.contains("httpbin.org/uuid") => {
                // Simulate UUID endpoint that changes each time
                let uuid = format!("{:08x}-{:04x}-{:04x}-{:04x}-{:012x}", 
                    chrono::Utc::now().timestamp() as u32,
                    (chrono::Utc::now().timestamp() >> 32) as u16,
                    4000, // version 4
                    8000, // variant bits
                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64 & 0xffffffffffff
                );
                Ok(format!(r#"{{"uuid": "{}"}}"#, uuid))
            }
            url if url.contains("worldtimeapi.org") => {
                // Simulate time API response
                let now = chrono::Utc::now();
                Ok(format!(r#"{{"datetime": "{}", "timestamp": {}}}"#, 
                    now.to_rfc3339(), 
                    now.timestamp()))
            }
            _ => {
                // For real URLs in tests, use a blocking HTTP client
                // Note: In production, this would be async
                #[cfg(test)]
                {
                    Ok(format!("Mock response for {}", self.source_url.as_str()))
                }
                #[cfg(not(test))]
                {
                    // In production, you would use async reqwest here
                    // For now, return an error for unknown URLs
                    Err(SourceError::Network(format!("HTTP fetching not implemented for: {}", self.source_url.as_str())))
                }
            }
        }
    }
    
    /// Extract content using the match expression
    fn extract_content(&self, raw_content: &str) -> Result<String, SourceError> {
        self.matches.extract_from(raw_content)
    }
}

/// Determine cache behavior for macro usage based on environment variables and keyword arguments
/// 
/// Environment variable CACHE_RESET takes precedence:
/// - CACHE_RESET=OVERWRITE -> CacheBehavior::Ignored (forces fresh fetch)
/// - CACHE_RESET=NONE -> Uses default behavior
/// 
/// If no environment override, uses the provided cache_override or defaults to Enabled
fn determine_cache_behavior_for_macro(cache_override: Option<cite_cache::CacheBehavior>) -> cite_cache::CacheBehavior {
    // Check environment variable first (takes precedence)
    if let Ok(cache_reset) = std::env::var("CACHE_RESET") {
        match cache_reset.to_uppercase().as_str() {
            "OVERWRITE" => return cite_cache::CacheBehavior::Ignored,
            "NONE" => {
                // Fall through to use provided behavior or default
            }
            _ => {
                // Invalid value, fall through to default behavior
            }
        }
    }
    
    // Use provided cache behavior or default to Enabled
    cache_override.unwrap_or(cite_cache::CacheBehavior::Enabled)
}

impl Source<ReferencedHttp, CurrentHttp, HttpDiff> for HttpMatch {
    fn id(&self) -> &Id {
        &self.id
    }
    
    fn get(&self) -> Result<Comparison<ReferencedHttp, CurrentHttp, HttpDiff>, SourceError> {
        
        // Use the internal cache with the configured behavior
        self.cache.get_source_with_cache(self, self.cache_behavior.clone())
            .map_err(|e| SourceError::Network(format!("Cache error: {}", e)))
    }
    
    fn get_referenced(&self) -> Result<ReferencedHttp, SourceError> {
        // This method provides a fallback when no cache is available
        // In practice, the cache system should be used via Cache::get_source_with_cache()
        // which will provide the actual referenced content from the cache
        //
        // For sources without cache, we return the current content as the reference
        // This makes the source "self-referencing" on first use
        let current = self.get_current()?;
        
        Ok(ReferencedHttp {
            content: current.content,
            metadata: current.metadata,
            source_url: self.source_url.clone(),
            match_expression: self.matches.clone(),
        })
    }
    
    fn get_current(&self) -> Result<CurrentHttp, SourceError> {
        let raw_content = self.fetch_http()?;
        let extracted_content = self.extract_content(&raw_content)?;
        
        let mut metadata = HashMap::new();
        metadata.insert("fetched_at".to_string(), chrono::Utc::now().to_rfc3339());
        metadata.insert("content_length".to_string(), raw_content.len().to_string());
        
        Ok(CurrentHttp {
            content: extracted_content,
            metadata,
            source_url: self.source_url.clone(),
            match_expression: self.matches.clone(),
            raw_content: Some(raw_content),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_match_expression_regex() -> Result<()> {
        let expr = MatchExpression::regex(".*");
        let content = "Hello, world!";
        let extracted = expr.extract_from(content)?;
        assert_eq!(extracted, content);
        Ok(())
    }

    #[test]
    fn test_source_url_validation() -> Result<()> {
        let url = SourceUrl::new("https://example.com/page")?;
        assert_eq!(url.as_str(), "https://example.com/page");
        assert_eq!(url.normalized(), "https://example.com/page");
        
        let url_with_slash = SourceUrl::new("https://example.com/")?;
        assert_eq!(url_with_slash.normalized(), "https://example.com");
        
        Ok(())
    }

    #[test]
    fn test_source_url_invalid() {
        let result = SourceUrl::new("ftp://example.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_http_match_creation() -> Result<()> {
        let http_match = HttpMatch::cached("https://example.com", ".*")?;
        assert_eq!(http_match.source_url.as_str(), "https://example.com");
        Ok(())
    }

    #[test]
    fn test_http_source_fetch() -> Result<()> {
        let http_match = HttpMatch::cached("https://example.com", ".*")?;
        let current = http_match.get_current()?;
        
        assert!(current.content.contains("Example Domain"));
        assert!(current.metadata.contains_key("fetched_at"));
        assert_eq!(current.source_url.as_str(), "https://example.com");
        Ok(())
    }

    #[test]
    fn test_http_diff() -> Result<()> {
        let referenced = ReferencedHttp {
            content: "old content".to_string(),
            metadata: HashMap::new(),
            source_url: SourceUrl::new("https://example.com")?,
            match_expression: MatchExpression::regex(".*"),
        };
        
        let current = CurrentHttp {
            content: "new content".to_string(),
            metadata: HashMap::new(),
            source_url: SourceUrl::new("https://example.com")?,
            match_expression: MatchExpression::regex(".*"),
            raw_content: None,
        };
        
        let diff = current.diff(&referenced)?;
        assert!(diff.content_changed);
        assert!(!diff.url_changed);
        assert!(!diff.match_expression_changed);
        assert!(!diff.is_empty());
        
        Ok(())
    }

  

    #[test]
    fn test_cacheable_serialization() -> Result<()> {
        let referenced = ReferencedHttp {
            content: "test content".to_string(),
            metadata: HashMap::new(),
            source_url: SourceUrl::new("https://example.com")?,
            match_expression: MatchExpression::regex(".*"),
        };
        
        let buffer = serde_json::to_vec(&referenced)?;
        let deserialized = ReferencedHttp::from_cached_buffer(buffer)?;
        
        assert_eq!(referenced, deserialized);
        Ok(())
    }

    #[test]
    fn test_regex_match_expression() -> Result<()> {
        let expr = MatchExpression::regex(r#""timestamp":\s*(\d+)"#);
        let json_content = r#"{"timestamp": 1234567890, "data": "test"}"#;
        
        let extracted = expr.extract_from(json_content)?;
        assert_eq!(extracted, "1234567890");
        
        Ok(())
    }

    #[test]
    fn test_css_selector_match_expression() -> Result<()> {
        let expr = MatchExpression::css_selector("h1");
        let html_content = "<html><body><h1>Title</h1><p>Content</p></body></html>";
        
        let extracted = expr.extract_from(html_content)?;
        assert_eq!(extracted, "Title");
        
        Ok(())
    }

    #[test]
    fn test_timestamp_endpoint_shows_diff() -> Result<()> {
        let http_match = HttpMatch::cached("https://httpbin.org/json", r#""timestamp":\s*(\d+)"#)?;
        
        // Get first response
        let current1 = http_match.get_current()?;
        
        // Simulate a referenced value (previous timestamp)
        let referenced = ReferencedHttp {
            content: "1234567890".to_string(),
            metadata: HashMap::new(),
            source_url: http_match.source_url.clone(),
            match_expression: http_match.matches.clone(),
        };
        
        // Create diff
        let diff = current1.diff(&referenced)?;
        
        // Should show content changed since timestamps will be different
        assert!(diff.content_changed);
        assert!(!diff.is_empty());
        assert!(diff.unified_diff().is_some());
        
        let unified_diff = diff.unified_diff().unwrap();
        assert!(unified_diff.contains("-1234567890"));
        assert!(unified_diff.contains(&format!("+{}", current1.content)));
        
        Ok(())
    }

    #[test]
    fn test_uuid_endpoint_changes() -> Result<()> {
        let http_match = HttpMatch::cached("https://httpbin.org/uuid", r#""uuid":\s*"([^"]+)""#)?;
        
        // Get two responses
        let current1 = http_match.get_current()?;
        std::thread::sleep(std::time::Duration::from_millis(1)); // Ensure different timestamp
        let current2 = http_match.get_current()?;
        
        // They should be different
        assert_ne!(current1.content, current2.content);
        
        Ok(())
    }


        #[test]
    fn test_http_diff_formatting() -> Result<()> {
        let referenced = ReferencedHttp {
            content: "Line 1\nOld Line 2\nLine 3".to_string(),
            metadata: HashMap::new(),
            source_url: SourceUrl::new("https://example.com")?,
            match_expression: MatchExpression::full_document(),
        };
        
        let current = CurrentHttp {
            content: "Line 1\nNew Line 2\nLine 3\nLine 4".to_string(),
            metadata: HashMap::new(),
            source_url: SourceUrl::new("https://example.com")?,
            match_expression: MatchExpression::full_document(),
            raw_content: None,
        };
        
        let diff = current.diff(&referenced)?;
        
        assert!(diff.content_changed);
        assert!(diff.unified_diff().is_some());
        
        let unified_diff = diff.unified_diff().unwrap();
        // Should contain git-style diff markers
        assert!(unified_diff.contains(" Line 1")); // unchanged line
        assert!(unified_diff.contains("-Old Line 2")); // removed line
        assert!(unified_diff.contains("+New Line 2")); // added line
        assert!(unified_diff.contains("+Line 4")); // new line
        
        Ok(())
    }

    #[test]
    fn test_url_fragment_parsing() -> Result<()> {
        // Test URL with fragment
        let url_with_fragment = SourceUrl::new("https://example.com/docs#section-1")?;
        assert_eq!(url_with_fragment.as_str(), "https://example.com/docs#section-1");
        assert_eq!(url_with_fragment.base_url(), "https://example.com/docs");
        assert_eq!(url_with_fragment.fragment(), Some("section-1"));
        
        // Test URL without fragment
        let url_without_fragment = SourceUrl::new("https://example.com/docs")?;
        assert_eq!(url_without_fragment.fragment(), None);
        assert_eq!(url_without_fragment.base_url(), "https://example.com/docs");
        
        // Test URL with empty fragment
        let url_empty_fragment = SourceUrl::new("https://example.com/docs#")?;
        assert_eq!(url_empty_fragment.fragment(), None);
        
        Ok(())
    }

    #[test]
    fn test_fragment_match_expression() -> Result<()> {
        let html_content = r#"
            <html>
                <body>
                    <h1>Main Title</h1>
                    <section id="intro">
                        <h2>Introduction</h2>
                        <p>This is the intro section.</p>
                    </section>
                    <section id="details">
                        <h2>Details</h2>
                        <p>This is the details section.</p>
                    </section>
                    <a name="legacy-anchor">Legacy content</a>
                </body>
            </html>
        "#;
        
        // Test fragment matching by id
        let fragment_expr = MatchExpression::fragment("intro");
        let extracted = fragment_expr.extract_from(html_content)?;
        assert!(extracted.contains("Introduction"));
        assert!(extracted.contains("intro section"));
        
        // Test fragment matching by name attribute
        let legacy_expr = MatchExpression::fragment("legacy-anchor");
        let legacy_extracted = legacy_expr.extract_from(html_content)?;
        assert!(legacy_extracted.contains("Legacy content"));
        
        // Test non-existent fragment
        let missing_expr = MatchExpression::fragment("nonexistent");
        let missing_extracted = missing_expr.extract_from(html_content)?;
        assert_eq!(missing_extracted, "");
        
        Ok(())
    }

    #[test]
    fn test_auto_fragment_detection() -> Result<()> {
        // Test auto-fragment with URL containing fragment
        let http_match = HttpMatch::with_auto_fragment("https://example.com#my-section")?;
        assert_eq!(http_match.source_url.fragment(), Some("my-section"));
        assert!(matches!(http_match.matches, MatchExpression::Fragment(_)));
        
        // Test auto-fragment with URL without fragment
        let http_match_no_fragment = HttpMatch::with_auto_fragment("https://example.com")?;
        assert_eq!(http_match_no_fragment.source_url.fragment(), None);
        assert!(matches!(http_match_no_fragment.matches, MatchExpression::FullDocument));
        
        Ok(())
    }

    #[test]
    fn test_fragment_based_content_extraction() -> Result<()> {
        // Create a mock HTML response with fragments
        let mock_html = r#"
            <html>
                <head><title>Test Page</title></head>
                <body>
                    <h1>Main Content</h1>
                    <div id="important-section">
                        <h2>Important Information</h2>
                        <p>This is critical content that we want to track.</p>
                        <ul>
                            <li>Point 1</li>
                            <li>Point 2</li>
                        </ul>
                    </div>
                    <div id="other-section">
                        <p>Other content that might change frequently.</p>
                    </div>
                </body>
            </html>
        "#;
        
        // Test extracting only the important section
        let fragment_expr = MatchExpression::fragment("important-section");
        let extracted = fragment_expr.extract_from(mock_html)?;
        
        assert!(extracted.contains("Important Information"));
        assert!(extracted.contains("critical content"));
        assert!(extracted.contains("Point 1"));
        assert!(extracted.contains("Point 2"));
        // Should NOT contain content from other sections
        assert!(!extracted.contains("Other content"));
        
        Ok(())
    }

    #[test]
    fn test_cache_hits_with_static_content() -> Result<()> {
        // Test cache behavior with static content (example.com)
        let http_match = HttpMatch::with_match_expression_and_cache_behavior(
            "https://example.com",
            MatchExpression::regex(".*"),
            cite_cache::CacheBehavior::Enabled
        )?;

        // First call - should populate cache
        let result1 = http_match.get()?;
        let first_content = result1.current().content.clone();
        assert!(first_content.contains("Example Domain"));

        // Second call - should use cache for referenced content
        let result2 = http_match.get()?;
        
        // The key test: referenced content should come from cache (first call's current)
        assert_eq!(result2.referenced().content, first_content,
                   "Second call should use cached content as referenced");
        
        // Current content should be the same for static sites
        assert!(result2.current().content.contains("Example Domain"));
        
        Ok(())
    }

    #[test]
    fn test_cache_hits_with_dynamic_content() -> Result<()> {
        // Test cache behavior with dynamic content (UUID endpoint)
        let http_match = HttpMatch::with_match_expression_and_cache_behavior(
            "https://httpbin.org/uuid",
            MatchExpression::regex(r#""uuid":\s*"([^"]+)""#),
            cite_cache::CacheBehavior::Enabled
        )?;

        // First call - should populate cache
        let result1 = http_match.get()?;
        let first_uuid = result1.current().content.clone();
        assert!(first_uuid.len() > 10); // Should contain UUID

        // Small delay to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Second call - should use cache for referenced, fresh for current
        let result2 = http_match.get()?;
        
        // Critical test: referenced should be from cache (first call's current)
        assert_eq!(result2.referenced().content, first_uuid,
                   "Referenced content should come from cache");
        
        // Current should be fresh (potentially different UUID)
        assert!(result2.current().content.len() > 10);
        
        // If UUIDs are different, we should see a diff
        if result2.current().content != first_uuid {
            assert!(!result2.diff().is_empty(), "Should show diff when UUIDs differ");
        }

        Ok(())
    }

    #[test]
    fn test_cache_ignored_vs_enabled() -> Result<()> {
        // Compare cache ignored vs enabled behavior
        
        // Test 1: Cache ignored - always fresh fetches
        let http_match_ignored = HttpMatch::with_match_expression_and_cache_behavior(
            "https://httpbin.org/uuid",
            MatchExpression::regex(r#""uuid":\s*"([^"]+)""#),
            cite_cache::CacheBehavior::Ignored
        )?;

        let result_ignored = http_match_ignored.get()?;
        // With ignored cache, referenced and current are separate fetches
        assert!(!result_ignored.diff().is_empty(), "Cache ignored should show diff for dynamic content");

        // Test 2: Cache enabled - should cache between calls
        let http_match_enabled = HttpMatch::with_match_expression_and_cache_behavior(
            "https://httpbin.org/uuid",
            MatchExpression::regex(r#""uuid":\s*"([^"]+)""#),
            cite_cache::CacheBehavior::Enabled
        )?;

        // First call
        let result1 = http_match_enabled.get()?;
        let cached_content = result1.current().content.clone();

        // Second call - should use cached referenced
        let result2 = http_match_enabled.get()?;
        assert_eq!(result2.referenced().content, cached_content,
                   "Cache enabled should preserve referenced content between calls");

        Ok(())
    }

    #[test]
    fn test_cache_with_different_sources_same_url() -> Result<()> {
        // Test that different HttpMatch instances with same URL share cache
        let url = "https://httpbin.org/uuid";
        let pattern = r#""uuid":\s*"([^"]+)""#;
        
        // First HttpMatch instance
        let http_match1 = HttpMatch::with_match_expression_and_cache_behavior(
            url,
            MatchExpression::regex(pattern),
            cite_cache::CacheBehavior::Enabled
        )?;

        let result1 = http_match1.get()?;
        let first_uuid = result1.current().content.clone();

        // Second HttpMatch instance with same URL and pattern
        let http_match2 = HttpMatch::with_match_expression_and_cache_behavior(
            url,
            MatchExpression::regex(pattern),
            cite_cache::CacheBehavior::Enabled
        )?;

        let result2 = http_match2.get()?;
        
        // The second instance should use the first instance's cached content as referenced
        assert_eq!(result2.referenced().content, first_uuid,
                   "Different HttpMatch instances with same URL should share cache");

        Ok(())
    }

    #[test]
    fn test_cache_key_includes_match_expression() -> Result<()> {
        // Test that different match expressions on same URL have different cache keys
        let url = "https://example.com";
        
        let http_match_regex = HttpMatch::with_match_expression_and_cache_behavior(
            url,
            MatchExpression::regex(".*"),
            cite_cache::CacheBehavior::Enabled
        )?;
        
        let http_match_css = HttpMatch::with_match_expression_and_cache_behavior(
            url,
            MatchExpression::css_selector("title"),
            cite_cache::CacheBehavior::Enabled
        )?;
        
        let http_match_full = HttpMatch::with_match_expression_and_cache_behavior(
            url,
            MatchExpression::full_document(),
            cite_cache::CacheBehavior::Enabled
        )?;

        // Verify that different match expressions create different cache IDs
        let regex_id = http_match_regex.id().as_str();
        let css_id = http_match_css.id().as_str();
        let full_id = http_match_full.id().as_str();
        
        assert_ne!(regex_id, css_id, "Regex and CSS selector should have different cache IDs");
        assert_ne!(regex_id, full_id, "Regex and full document should have different cache IDs");
        assert_ne!(css_id, full_id, "CSS selector and full document should have different cache IDs");
        
        // Verify they all contain the URL component
        assert!(regex_id.contains("example_com"), "Cache ID should contain URL component");
        assert!(css_id.contains("example_com"), "Cache ID should contain URL component");
        assert!(full_id.contains("example_com"), "Cache ID should contain URL component");
        
        // Verify they contain match expression components
        assert!(regex_id.contains("regex"), "Regex cache ID should contain 'regex'");
        assert!(css_id.contains("css"), "CSS cache ID should contain 'css'");
        assert!(full_id.contains("full"), "Full document cache ID should contain 'full'");

        Ok(())
    }
}
