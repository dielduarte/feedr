use tauri::Url;

/// Whether `target` is part of the app. Everything else belongs in the default browser, so the
/// window only ever shows feedrsauros.
pub fn is_app_url(app: &Url, target: &Url) -> bool {
    target.origin() == app.origin()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        s.parse().unwrap()
    }

    #[test]
    fn pages_of_the_app_stay_in_the_window() {
        let app = url("http://127.0.0.1:53211/");

        assert!(is_app_url(&app, &url("http://127.0.0.1:53211/feeds/cloudflare-blog")));
    }

    #[test]
    fn other_sites_leave_the_window() {
        let app = url("http://127.0.0.1:53211/");

        assert!(!is_app_url(&app, &url("https://blog.cloudflare.com/some-post")));
        assert!(!is_app_url(&app, &url("http://127.0.0.1:7777/")));
        assert!(!is_app_url(&app, &url("mailto:someone@example.com")));
    }
}
