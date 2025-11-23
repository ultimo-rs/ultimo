# Ultimo Website Deployment

Marketing website for the Ultimo Rust web framework.

## Cloudflare Pages Configuration

### Build Settings

- **Framework preset**: Next.js
- **Build command**: `pnpm build`
- **Build output directory**: `.next`
- **Root directory**: `website`
- **Node.js version**: 22

### Environment Variables

No environment variables required for production build.

## Local Development

```bash
# Install dependencies
pnpm install

# Run dev server
pnpm dev

# Or use moonrepo
moon run website:dev
```

Site runs at http://localhost:3000

## Tech Stack

- Next.js 16 with Turbopack
- Shadcn/UI + Tailwind CSS v4
- Lucide React icons
- Deployed on Cloudflare Pages
