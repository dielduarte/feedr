use ammonia::UrlRelative;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
pub struct SanitizedHtml(String);

impl SanitizedHtml {
    pub fn clean(raw: &str, base: &Url) -> Self {
        let html = ammonia::Builder::default()
            .url_relative(UrlRelative::RewriteWithBase(base.clone()))
            .clean(raw)
            .to_string();
        Self(html)
    }

    /// Content is cleaned before it is stored, so reading it back needs no second pass.
    pub(crate) fn from_stored(html: String) -> Self {
        Self(html)
    }

    /// Plain text for previews: markup dropped, whitespace collapsed, cut at a word boundary.
    pub fn excerpt(&self, max_chars: usize) -> Option<String> {
        let fragment = scraper::Html::parse_fragment(&self.0);
        let words: Vec<&str> = fragment
            .root_element()
            .text()
            .flat_map(str::split_whitespace)
            .collect();
        let full = words.join(" ");
        if full.is_empty() {
            return None;
        }
        if full.chars().count() <= max_chars {
            return Some(full);
        }

        let budget = max_chars.saturating_sub(1);
        let mut excerpt = String::new();
        for word in words {
            let separator = usize::from(!excerpt.is_empty());
            if excerpt.chars().count() + separator + word.chars().count() > budget {
                break;
            }
            if separator == 1 {
                excerpt.push(' ');
            }
            excerpt.push_str(word);
        }
        if excerpt.is_empty() {
            excerpt = full.chars().take(budget).collect();
        }
        excerpt.push('…');
        Some(excerpt)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
