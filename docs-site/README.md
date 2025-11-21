# Ultimo Documentation Site

This is the documentation website for Ultimo, built with [Vocs](https://vocs.dev).

## Development

```bash
# Install dependencies
pnpm install

# Start dev server
pnpm dev

# Build for production
pnpm build

# Preview production build
pnpm preview
```

## Structure

```
docs-site/
├── docs/              # Markdown documentation files
│   ├── index.md       # Homepage (from README.md)
│   ├── getting-started.md
│   ├── quick-reference.md
│   ├── testing.md
│   ├── cli.md
│   ├── examples.md
│   ├── core/          # Core concepts
│   │   └── routing.md
│   ├── features/      # Feature guides
│   │   ├── database.md
│   │   ├── rpc.md
│   │   ├── openapi.md
│   │   ├── typescript.md
│   │   └── headers.md
│   └── comparisons/   # Comparisons & benchmarks
│       ├── rpc.md
│       └── benchmarks.md
├── vocs.config.ts     # Vocs configuration
└── package.json
```

## Deployment

The site can be deployed to Vercel, Netlify, or GitHub Pages. Build with `pnpm build` and deploy the `dist/` directory.
