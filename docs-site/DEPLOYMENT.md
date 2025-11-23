# Cloudflare Pages Deployment

This docs site is built with [Vocs](https://vocs.dev) and deployed to Cloudflare Pages.

## Automatic Deployment

Pushes to `main` branch automatically deploy to: https://ultimo.pages.dev

## Cloudflare Pages Settings

**Build Configuration:**

- Framework preset: `None` (Vite)
- Build command: `pnpm install && pnpm build`
- Build output directory: `docs/.vocs/dist` ⚠️ **Important:** This path is relative to the root directory
- Root directory: `docs-site`

**Environment Variables:**

- `NODE_VERSION`: `22` (required for Vocs globSync support)
- `PNPM_VERSION`: `latest`

**Note:** The `.node-version` file in this directory specifies Node 22, which Cloudflare Pages will use automatically.

## Local Development

```bash
cd docs-site
pnpm install
pnpm dev
```

## Manual Build

```bash
cd docs-site
pnpm build
pnpm preview
```

## Custom Domain Setup

1. Go to Cloudflare Pages project settings
2. Add custom domain: `docs.ultimo.dev`
3. Cloudflare will automatically configure DNS
