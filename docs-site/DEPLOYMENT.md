# Cloudflare Pages Deployment

This docs site is built with [Vocs](https://vocs.dev) and deployed to Cloudflare Pages.

## Automatic Deployment

Pushes to `main` branch automatically deploy to: https://ultimo.pages.dev

## Cloudflare Pages Settings

**Build Configuration:**
- Framework preset: `None` (Vite)
- Build command: `pnpm install && pnpm build`
- Build output directory: `docs/.vocs/dist`
- Root directory: `docs-site`

**Environment Variables:**
- `NODE_VERSION`: `20`
- `PNPM_VERSION`: `latest`

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
