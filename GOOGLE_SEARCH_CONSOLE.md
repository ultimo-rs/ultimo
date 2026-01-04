# Google Search Console Verification Guide

## Quick Setup for Ultimo Websites

### Properties to Add
1. **Main Website**: https://ultimo.dev
2. **Documentation**: https://docs.ultimo.dev

## Verification Methods

### Option 1: DNS Verification (Recommended - Verifies All Subdomains)

1. Go to [Google Search Console](https://search.google.com/search-console)
2. Click "Add Property"
3. Choose "Domain" property type
4. Enter: `ultimo.dev`
5. Google will provide a TXT record like:
   ```
   google-site-verification=abc123xyz456
   ```
6. Add this TXT record to your DNS settings (where your domain is registered)
7. Wait for DNS propagation (can take a few minutes to 48 hours)
8. Click "Verify" in Google Search Console

**Benefits**: This verifies both `ultimo.dev` and `docs.ultimo.dev` at once!

### Option 2: HTML File Upload

1. Go to [Google Search Console](https://search.google.com/search-console)
2. Click "Add Property"
3. Choose "URL prefix" property type
4. Enter: `https://ultimo.dev`
5. Choose "HTML file" verification method
6. Download the verification file (e.g., `google1234567890abcdef.html`)

**For Main Website:**
- Place file in `/website/public/google1234567890abcdef.html`
- It will be accessible at `https://ultimo.dev/google1234567890abcdef.html`

**For Docs Website:**
- Place file in `/docs-site/docs/public/google1234567890abcdef.html`
- It will be accessible at `https://docs.ultimo.dev/google1234567890abcdef.html`

7. Deploy your changes
8. Click "Verify" in Google Search Console

### Option 3: HTML Meta Tag (Already Prepared)

1. Go to [Google Search Console](https://search.google.com/search-console)
2. Choose "HTML tag" verification method
3. Copy the meta tag provided
4. Add it to your layout files:

**For Main Website** (`/website/app/layout.tsx`):
```tsx
// Add to the head section
<meta name="google-site-verification" content="your-verification-code" />
```

**For Docs Website** (`/docs-site/vocs.config.ts`):
```typescript
head: {
  meta: [
    {
      name: "google-site-verification",
      content: "your-verification-code"
    },
    // ... other meta tags
  ]
}
```

5. Deploy and verify

## After Verification

### 1. Submit Your Sitemaps

**Main Website:**
- URL: `https://ultimo.dev/sitemap.xml`
- Go to Search Console → Sitemaps → Add new sitemap
- Enter: `sitemap.xml`

**Docs Website:**
Vocs typically generates a sitemap automatically. Check if one exists at:
- `https://docs.ultimo.dev/sitemap.xml`

If not, you may need to add sitemap generation to vocs configuration.

### 2. Monitor Your Sites

In Google Search Console, you can:
- See search performance
- Monitor indexing status
- Check for crawl errors
- Submit URLs for indexing
- See which pages are indexed

### 3. Request Indexing

For important pages, you can request immediate indexing:
1. Go to URL Inspection tool
2. Enter your URL (e.g., `https://ultimo.dev`)
3. Click "Request Indexing"

## Troubleshooting

### Verification Failed
- Make sure files are deployed and accessible
- Check DNS propagation if using DNS method
- Clear CDN cache if using Vercel/Cloudflare
- Wait a few hours and try again

### Sitemap Not Found
- Check sitemap URL is accessible
- Ensure sitemap.xml is being generated
- Check robots.txt allows sitemap crawling

## Additional Setup (Already Done)

✅ robots.txt configured
✅ sitemap.xml generated automatically  
✅ Meta tags for SEO added
✅ OG images configured
✅ Structured data ready

## Next Steps

1. Choose a verification method (DNS recommended)
2. Complete verification for both domains
3. Submit sitemaps
4. Monitor search performance
5. Request indexing for key pages

## Resources

- [Google Search Console](https://search.google.com/search-console)
- [Search Console Help](https://support.google.com/webmasters)
- [Sitemap Protocol](https://www.sitemaps.org/)
