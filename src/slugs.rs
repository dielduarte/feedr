/// Long enough to read, short enough to share.
const MAX_LEN: usize = 80;

/// A lowercase, hyphenated URL segment for `text`, or `fallback` when nothing usable is left.
pub fn slugify(text: &str, fallback: &str) -> String {
    let full = slug::slugify(text);
    if full.is_empty() {
        return fallback.to_string();
    }
    if full.len() <= MAX_LEN {
        return full;
    }
    let mut short = String::new();
    for word in full.split('-') {
        let needed = if short.is_empty() {
            word.len()
        } else {
            word.len() + 1
        };
        if short.len() + needed > MAX_LEN {
            break;
        }
        if !short.is_empty() {
            short.push('-');
        }
        short.push_str(word);
    }
    if short.is_empty() {
        full[..MAX_LEN].to_string()
    } else {
        short
    }
}

/// `base`, or `base-2`, `base-3`… for the first one not already taken.
pub fn unique_slug(base: &str, is_taken: impl Fn(&str) -> bool) -> String {
    if !is_taken(base) {
        return base.to_string();
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|candidate| !is_taken(candidate))
        .expect("an unused suffix always exists")
}
