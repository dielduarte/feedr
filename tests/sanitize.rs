use feedrsauros::sanitize::SanitizedHtml;
use url::Url;

fn clean(html: &str) -> SanitizedHtml {
    SanitizedHtml::clean(html, &Url::parse("https://example.com/").unwrap())
}

#[test]
fn excerpts_are_plain_text_with_collapsed_whitespace() {
    let html = clean("<p>Hello&nbsp;<b>wide</b>\n\n  world &amp; friends</p><p>Again</p>");

    assert_eq!(
        html.excerpt(100).as_deref(),
        Some("Hello wide world & friends Again")
    );
}

#[test]
fn excerpts_never_include_script_contents() {
    let html = clean("<p>Visible</p><script>alert('hidden')</script><style>p{}</style>");

    assert_eq!(html.excerpt(100).as_deref(), Some("Visible"));
}

#[test]
fn long_excerpts_are_cut_at_a_word_boundary() {
    let html = clean("<p>one two three four five six</p>");

    assert_eq!(html.excerpt(12).as_deref(), Some("one two…"));
}

#[test]
fn empty_content_has_no_excerpt() {
    assert_eq!(clean("<p> </p><img src=\"a.png\">").excerpt(100), None);
}
