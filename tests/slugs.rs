use feedrsauros::slugs::{slugify, unique_slug};

#[test]
fn turns_titles_into_readable_url_segments() {
    assert_eq!(
        slugify(
            "Saving another 100TB of RAM with math (and Rust)",
            "article"
        ),
        "saving-another-100tb-of-ram-with-math-and-rust"
    );
    assert_eq!(slugify("  Cloudflare   Blog!! ", "feed"), "cloudflare-blog");
    assert_eq!(slugify("Café & Crème", "feed"), "cafe-creme");
}

#[test]
fn falls_back_when_nothing_usable_is_left() {
    assert_eq!(slugify("!!!", "feed"), "feed");
    assert_eq!(slugify("", "article"), "article");
}

#[test]
fn keeps_long_titles_short_at_a_word_boundary() {
    let slug = slugify(&"word ".repeat(40), "article");

    assert!(slug.len() <= 80, "{} chars", slug.len());
    assert!(!slug.ends_with('-'));
    assert!(slug.split('-').all(|part| part == "word"));
}

#[test]
fn numbers_duplicates_from_two() {
    let taken = ["rust-blog", "rust-blog-2"];
    let is_taken = |slug: &str| taken.contains(&slug);

    assert_eq!(unique_slug("rust-blog", is_taken), "rust-blog-3");
    assert_eq!(unique_slug("cloudflare-blog", is_taken), "cloudflare-blog");
}
