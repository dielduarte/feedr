use feedrsauros::discover::feed_links;
use url::Url;

fn links(html: &str) -> Vec<String> {
    let page = Url::parse("https://blog.example.com/posts/hello").unwrap();
    feed_links(html, &page)
        .into_iter()
        .map(String::from)
        .collect()
}

#[test]
fn finds_rss_atom_and_json_feeds_in_document_order() {
    let html = r#"<html><head>
        <link rel="alternate" type="application/atom+xml" href="https://blog.example.com/atom.xml">
        <link rel="alternate" type="application/rss+xml" href="https://blog.example.com/rss.xml">
        <link rel="alternate" type="application/feed+json" href="https://blog.example.com/feed.json">
    </head></html>"#;

    assert_eq!(
        links(html),
        [
            "https://blog.example.com/atom.xml",
            "https://blog.example.com/rss.xml",
            "https://blog.example.com/feed.json"
        ]
    );
}

#[test]
fn resolves_relative_hrefs_against_the_page() {
    let html = r#"<link rel="alternate" type="application/rss+xml" href="/feed.xml">
                  <link rel="alternate" type="application/rss+xml" href="comments.xml">"#;

    assert_eq!(
        links(html),
        [
            "https://blog.example.com/feed.xml",
            "https://blog.example.com/posts/comments.xml"
        ]
    );
}

#[test]
fn matches_rel_and_type_case_insensitively_among_other_tokens() {
    let html = r#"<link rel="Feed ALTERNATE" type="Application/RSS+XML" href="/feed.xml">"#;

    assert_eq!(links(html), ["https://blog.example.com/feed.xml"]);
}

#[test]
fn ignores_links_that_are_not_feeds() {
    let html = r#"
        <link rel="alternate" hreflang="pt" type="text/html" href="/pt/">
        <link rel="stylesheet" type="text/css" href="/style.css">
        <link rel="icon" href="/favicon.ico">
        <link rel="alternate" type="application/rss+xml">
        <a href="/feed.xml">RSS</a>"#;

    assert!(links(html).is_empty());
}
