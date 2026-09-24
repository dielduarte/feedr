# feedr

A local-first RSS reader: one small program that fetches your feeds in the background and serves a clean web app to read them.

- Follows RSS, Atom and JSON Feed. Paste a site's address and feedr finds its feed.
- Organise feeds into folders by dragging them in the sidebar.
- Checks busy feeds more often and quiet ones less, and backs off politely when sites ask it to.
- Updates the page live as new articles arrive.
- Imports and exports OPML, so you can move from or to any other reader.

## Run it on your computer

Requires [Rust](https://rustup.rs) and [pnpm](https://pnpm.io).

```bash
pnpm --dir web install && pnpm --dir web build
cargo install --path .
feedr serve --open
```

This opens http://127.0.0.1:7777. feedr fetches feeds while `feedr serve` is running and catches up when you start it again. Your data is stored in your user data directory (on macOS, `~/Library/Application Support/feedr/feedr.db`); pass `--db` to use another file.

Everything also works from the terminal:

```bash
feedr add jvns.ca --folder Blogs   # subscribe to a site or feed
feedr ls                           # folders, feeds and unread counts
feedr refresh                      # fetch every feed now
feedr import subscriptions.opml
feedr export > subscriptions.opml
```

## Self-host with Docker

```bash
docker compose up -d
```

Then open http://127.0.0.1:7777. Articles and subscriptions are kept in the `feedr-data` volume, so they survive restarts and upgrades. To upgrade, pull the latest code and run `docker compose up -d --build`.

Run CLI commands inside the container with `docker compose exec`:

```bash
docker compose exec feedr feedr add jvns.ca --folder Blogs
docker compose exec feedr feedr export > subscriptions.opml
```

### Reaching it from other devices

feedr has no login: anyone who can reach it can read and change your subscriptions. That is why `compose.yaml` only publishes it on `127.0.0.1`. To use it from your phone or another computer, put it behind a reverse proxy that adds authentication and HTTPS. With [Caddy](https://caddyserver.com), for example:

```caddy
feeds.example.com {
	basic_auth {
		# Generate the hash with: caddy hash-password
		you $2a$14$replace-with-your-password-hash
	}
	reverse_proxy 127.0.0.1:7777
}
```

A VPN such as Tailscale works too: keep the port private and reach the machine over the VPN.

### Configuration

| Variable | Default | What it does |
| --- | --- | --- |
| `FEEDR_PORT` | `7777` | Port on the host (in `compose.yaml`) or the port `feedr serve` listens on. |
| `FEEDR_HOST` | `127.0.0.1` (`0.0.0.0` in the container) | Address `feedr serve` listens on. |
| `FEEDR_DB` | your data directory (`/data/feedr.db` in the container) | Database file. |
| `RUST_LOG` | `feedr=info` | Log detail, for example `feedr=debug`. |

For example, `FEEDR_PORT=8080 docker compose up -d` serves feedr on http://127.0.0.1:8080.

## Development

```bash
cargo test                    # backend tests
pnpm --dir web test           # frontend unit tests
feedr serve                   # API on :7777 …
pnpm --dir web dev            # … and the web app with hot reload, proxying /api to it
```
