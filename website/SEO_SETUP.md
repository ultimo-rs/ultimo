# SEO Assets Setup Guide

## Files Created

### Website (https://ultimo.dev)
- `/app/sitemap.ts` - Auto-generated sitemap
- `/app/robots.ts` - Robots.txt configuration
- `/app/manifest.ts` - Web app manifest
- `/app/api/og/route.tsx` - Dynamic OG image generation with @vercel/og

### Docs Site (https://docs.ultimo.dev)
- Updated `vocs.config.ts` with meta tags and OG images

## Google Search Console Setup

To verify your sites in Google Search Console:

1. Go to https://search.google.com/search-console
2. Add properties for:
   - https://ultimo.dev
   - https://docs.ultimo.dev

3. **Verification Options:**

   **Option A: DNS Verification (Recommended)**
   - Choose DNS verification
   - Add the TXT record to your domain DNS settings
   - This verifies both domains at once

   **Option B: HTML File Upload**
   - Download the verification file (e.g., `google123abc.html`)
   - For website: Place in `/website/public/google123abc.html`
   - For docs: Place in `/docs-site/docs/public/google123abc.html`

   **Option C: Meta Tag Verification**
   - Add the meta tag to the head section of your layout files (already configured above)

4. After verification, submit your sitemaps:
   - Website: `https://ultimo.dev/sitemap.xml`
   - Docs: Configure Vocs to generate sitemap if needed

## Favicon and Icon Assets

You need to generate and add these icon files:

### For Website (`/website/public/`):
- `favicon.ico` - 32x32 favicon
- `icon-192.png` - 192x192 PNG icon
- `icon-512.png` - 512x512 PNG icon
- `apple-icon.png` - 180x180 Apple touch icon

### For Docs Site (`/docs-site/docs/public/`):
- `favicon.ico` - 32x32 favicon
- `apple-icon.png` - 180x180 Apple touch icon

### Recommended Tools for Icon Generation:

1. **Favicon Generator** - https://realfavicongenerator.net/
   - Upload your logo/design
   - Generates all required sizes
   - Provides optimal formats for all platforms

2. **Favicon.io** - https://favicon.io/
   - Simple favicon generator
   - Can generate from text, image, or emoji

3. **ImageMagick** (Command line):
   ```bash
   # If you have a source image
   convert source.png -resize 512x512 icon-512.png
   convert source.png -resize 192x192 icon-192.png
   convert source.png -resize 180x180 apple-icon.png
   convert source.png -resize 32x32 favicon.ico
   ```

4. **Figma/Sketch/Photoshop**
   - Design your icon
   - Export in multiple sizes

## OG Image Testing

Test your OG images:
- https://www.opengraph.xyz/
- https://cards-dev.twitter.com/validator

## Next Steps

1. Generate favicon and icon files using one of the tools above
2. Place them in the `/website/public/` and `/docs-site/docs/public/` directories
3. Verify your domains in Google Search Console
4. Submit your sitemaps
5. Test OG images on social media platforms
6. Deploy to production

## Deployment Notes

- Ensure all public assets are included in your deployment
- Verify that the OG image API route works in production
- Check that sitemap.xml and robots.txt are accessible
- Monitor Google Search Console for any crawl errors
