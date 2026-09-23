use ammonia::UrlRelative;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedHtml(String);

impl SanitizedHtml {
    pub fn clean(raw: &str, base: &Url) -> Self {
        let html = ammonia::Builder::default()
            .url_relative(UrlRelative::RewriteWithBase(base.clone()))
            .clean(raw)
            .to_string();
        Self(html)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
