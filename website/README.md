# feedrsauros.com

The landing page: plain HTML and CSS in `public/`, deployed to Cloudflare with [SST](https://sst.dev).

## Develop

```bash
pnpm install
pnpm dev        # http://localhost:8080, reloads on save
```

## Deploy

SST needs a Cloudflare API token. In the Cloudflare dashboard, create one under **Manage Account → API Tokens** from the **Edit Cloudflare Workers** template, and add **Zone → DNS → Edit** so it can attach `feedrsauros.com`. Then:

```bash
export CLOUDFLARE_API_TOKEN=...
export CLOUDFLARE_DEFAULT_ACCOUNT_ID=...
pnpm run deploy
```

This publishes `public/` to `feedrsauros.com`, with `www.feedrsauros.com` redirecting to it. Deploying any other stage (for example `pnpm sst deploy --stage preview`) gets its own `workers.dev` URL instead of the domain, so you can check changes before they go live.
