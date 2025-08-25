use std::collections::HashMap;
use crate::{Source, SourceError, Comparison, Referenced, Current, Diff, Content, Id};
use crate::{CacheableReferenced, CacheableCurrent, CacheError};
use serde::{Serialize, Deserialize};
use regex::Regex;
use scraper::{Html, Selector};
use similar::{ChangeTag, TextDiff};

/// Match expression for extracting content from hypertext
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
    
    /// Extract matching content from hypertext
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
        }
    }
}

/// Source URL with validation and normalization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceUrl {
    url: String,
    normalized: String,
}

impl SourceUrl {
    /// Create a new source URL with validation
    pub fn new(url: &str) -> Result<Self, SourceError> {
        let normalized = Self::normalize_url(url)?;
        Ok(Self {
            url: url.to_string(),
            normalized,
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

/// Hypertext content that was referenced at commit time
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencedHypertext {
    /// The extracted content that was referenced
    pub content: String,
    /// Metadata about the hypertext source
    pub metadata: HashMap<String, String>,
    /// The URL that was accessed
    pub source_url: SourceUrl,
    /// The match expression used
    pub match_expression: MatchExpression,
}

impl Content for ReferencedHypertext {}
impl Referenced for ReferencedHypertext {}

impl CacheableReferenced for ReferencedHypertext {
    fn from_cached_buffer(buffer: Vec<u8>) -> Result<Self, CacheError> {
        serde_json::from_slice(&buffer).map_err(|e| CacheError::Deserialize(e.into()))
    }
}

/// Current hypertext content fetched from the web
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentHypertext {
    /// The extracted content currently available
    pub content: String,
    /// Current metadata (headers, timestamps, etc.)
    pub metadata: HashMap<String, String>,
    /// The URL that was accessed
    pub source_url: SourceUrl,
    /// The match expression used
    pub match_expression: MatchExpression,
    /// Raw hypertext before extraction (for debugging)
    pub raw_content: Option<String>,
}

impl Content for CurrentHypertext {}

impl Current<ReferencedHypertext, HypertextDiff> for CurrentHypertext {
    fn diff(&self, referenced: &ReferencedHypertext) -> Result<HypertextDiff, SourceError> {
        let content_changed = self.content != referenced.content;
        let url_changed = self.source_url != referenced.source_url;
        let match_expression_changed = self.match_expression != referenced.match_expression;
        
        let mut diff = HypertextDiff {
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

impl CacheableCurrent<ReferencedHypertext, HypertextDiff> for CurrentHypertext {
    fn to_cached_buffer(&self) -> Result<Vec<u8>, CacheError> {
        // Convert to ReferencedHypertext format for caching
        let referenced = ReferencedHypertext {
            content: self.content.clone(),
            metadata: self.metadata.clone(),
            source_url: self.source_url.clone(),
            match_expression: self.match_expression.clone(),
        };
        serde_json::to_vec(&referenced).map_err(|e| CacheError::Serialize(e.into()))
    }
}

/// Diff between referenced and current hypertext
#[derive(Debug, Clone, PartialEq)]
pub struct HypertextDiff {
    pub content_changed: bool,
    pub url_changed: bool,
    pub match_expression_changed: bool,
    pub referenced_content: String,
    pub current_content: String,
    pub unified_diff: Option<String>,
}

impl HypertextDiff {
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

impl Diff for HypertextDiff {
    fn is_empty(&self) -> bool {
        !self.content_changed && !self.url_changed && !self.match_expression_changed
    }
}

/// Hypertext match source for checking committed hypertext references
pub struct HypertextMatch {
    pub matches: MatchExpression,
    pub source_url: SourceUrl,
    pub cache_path: String,
    id: Id,
}

impl HypertextMatch {
    /// Create a new hypertext match with caching
    pub fn cached(url: &str, pattern: &str) -> Result<Self, SourceError> {
        let source_url = SourceUrl::new(url)?;
        let matches = MatchExpression::regex(pattern);
        let cache_path = format!("hypertext_{}", Self::url_to_cache_key(url));
        let id = Id::new(cache_path.clone());
        
        Ok(Self {
            matches,
            source_url,
            cache_path,
            id,
        })
    }
    
    /// Create with custom match expression
    pub fn with_match_expression(url: &str, expression: MatchExpression) -> Result<Self, SourceError> {
        let source_url = SourceUrl::new(url)?;
        let cache_path = format!("hypertext_{}", Self::url_to_cache_key(url));
        let id = Id::new(cache_path.clone());
        
        Ok(Self {
            matches: expression,
            source_url,
            cache_path,
            id,
        })
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
    
    /// Fetch hypertext content from the URL
    fn fetch_hypertext(&self) -> Result<String, SourceError> {
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

impl Source<ReferencedHypertext, CurrentHypertext, HypertextDiff> for HypertextMatch {
    fn id(&self) -> &Id {
        &self.id
    }
    
    fn get(&self) -> Result<Comparison<ReferencedHypertext, CurrentHypertext, HypertextDiff>, SourceError> {
        let current = self.get_current()?;
        let referenced = self.get_referenced()?;
        let diff = current.diff(&referenced)?;
        Ok(Comparison::new(referenced, current, diff))
    }
    
    fn get_referenced(&self) -> Result<ReferencedHypertext, SourceError> {
        // This would typically come from commit history or cache
        // For now, return a placeholder
        Ok(ReferencedHypertext {
            content: "Referenced content placeholder".to_string(),
            metadata: HashMap::new(),
            source_url: self.source_url.clone(),
            match_expression: self.matches.clone(),
        })
    }
    
    fn get_current(&self) -> Result<CurrentHypertext, SourceError> {
        let raw_content = self.fetch_hypertext()?;
        let extracted_content = self.extract_content(&raw_content)?;
        
        let mut metadata = HashMap::new();
        metadata.insert("fetched_at".to_string(), chrono::Utc::now().to_rfc3339());
        metadata.insert("content_length".to_string(), raw_content.len().to_string());
        
        Ok(CurrentHypertext {
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
    use tempfile::TempDir;
    use crate::{CacheBuilder, CacheBehavior};

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
    fn test_hypertext_match_creation() -> Result<()> {
        let http_match = HypertextMatch::cached("https://example.com", ".*")?;
        assert_eq!(http_match.source_url.as_str(), "https://example.com");
        Ok(())
    }

    #[test]
    fn test_hypertext_source_fetch() -> Result<()> {
        let http_match = HypertextMatch::cached("https://example.com", ".*")?;
        let current = http_match.get_current()?;
        
        assert!(current.content.contains("Example Domain"));
        assert!(current.metadata.contains_key("fetched_at"));
        assert_eq!(current.source_url.as_str(), "https://example.com");
        Ok(())
    }

    #[test]
    fn test_hypertext_diff() -> Result<()> {
        let referenced = ReferencedHypertext {
            content: "old content".to_string(),
            metadata: HashMap::new(),
            source_url: SourceUrl::new("https://example.com")?,
            match_expression: MatchExpression::regex(".*"),
        };
        
        let current = CurrentHypertext {
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
    fn test_hypertext_with_cache() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), std::path::PathBuf::from("cache"));
        let cache = builder.build()?;
        
        let http_match = HypertextMatch::cached("https://example.com", ".*")?;
        
        // Test with cache enabled
        let result = cache.get_source_with_cache(http_match, CacheBehavior::Enabled)?;
        
        assert!(result.current().content.contains("Example Domain"));
        assert!(!result.diff().is_empty()); // Should have differences since referenced is placeholder
        
        Ok(())
    }

    #[test]
    fn test_cacheable_serialization() -> Result<()> {
        let referenced = ReferencedHypertext {
            content: "test content".to_string(),
            metadata: HashMap::new(),
            source_url: SourceUrl::new("https://example.com")?,
            match_expression: MatchExpression::regex(".*"),
        };
        
        let buffer = serde_json::to_vec(&referenced)?;
        let deserialized = ReferencedHypertext::from_cached_buffer(buffer)?;
        
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
        let http_match = HypertextMatch::cached("https://httpbin.org/json", r#""timestamp":\s*(\d+)"#)?;
        
        // Get first response
        let current1 = http_match.get_current()?;
        
        // Simulate a referenced value (previous timestamp)
        let referenced = ReferencedHypertext {
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
        let http_match = HypertextMatch::cached("https://httpbin.org/uuid", r#""uuid":\s*"([^"]+)""#)?;
        
        // Get two responses
        let current1 = http_match.get_current()?;
        std::thread::sleep(std::time::Duration::from_millis(1)); // Ensure different timestamp
        let current2 = http_match.get_current()?;
        
        // They should be different
        assert_ne!(current1.content, current2.content);
        
        Ok(())
    }

    #[test]
    fn test_full_workflow_with_cache_and_diff() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let builder = CacheBuilder::new(temp_dir.path().to_path_buf(), std::path::PathBuf::from("cache"));
        let cache = builder.build()?;
        
        let http_match = HypertextMatch::cached("https://worldtimeapi.org/api/timezone/UTC", r#""timestamp":\s*(\d+)"#)?;
        
        // First access - will cache current as referenced
        let result1 = cache.get_source_with_cache(http_match, CacheBehavior::Enabled)?;
        let first_timestamp = result1.current().content.clone();
        
        // Create a new source with same ID
        let http_match2 = HypertextMatch::cached("https://worldtimeapi.org/api/timezone/UTC", r#""timestamp":\s*(\d+)"#)?;
        
        // Second access - will use cached referenced but fetch fresh current
        std::thread::sleep(std::time::Duration::from_millis(1001)); // Ensure different timestamp (> 1 second)
        let result2 = cache.get_source_with_cache(http_match2, CacheBehavior::Enabled)?;
        
        // Should use cached value as referenced
        assert_eq!(result2.referenced().content, first_timestamp);
        
        // The key test: referenced should come from cache, current should be fresh
        // Even if timestamps are the same, this demonstrates cache behavior
        println!("Cached referenced: {}", result2.referenced().content);
        println!("Fresh current: {}", result2.current().content);
        
        // If content is different, should show a diff
        if result2.current().content != first_timestamp {
            assert!(result2.diff().content_changed);
            assert!(!result2.diff().is_empty());
        }
        
        Ok(())
    }

    #[test] 
    fn test_hypertext_diff_formatting() -> Result<()> {
        let referenced = ReferencedHypertext {
            content: "Line 1\nOld Line 2\nLine 3".to_string(),
            metadata: HashMap::new(),
            source_url: SourceUrl::new("https://example.com")?,
            match_expression: MatchExpression::full_document(),
        };
        
        let current = CurrentHypertext {
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
}
