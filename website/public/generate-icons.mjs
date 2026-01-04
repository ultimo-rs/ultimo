/**
 * Icon Generator for Ultimo
 * 
 * This script generates favicon and icon files from an SVG source.
 * 
 * Usage:
 *   1. Install sharp: npm install sharp (or it might already be installed)
 *   2. Run: node generate-icons.mjs
 * 
 * Alternative if sharp is not available:
 *   Use online tools:
 *   - https://realfavicongenerator.net/
 *   - https://favicon.io/
 */

import { readFileSync, existsSync } from 'fs';
import { join } from 'path';

console.log('🎨 Ultimo Icon Generation\n');

const source = existsSync('icon.svg') ? 'icon.svg' : existsSync('logo.svg') ? 'logo.svg' : null;

if (!source) {
  console.log('❌ No icon.svg or logo.svg found in public directory\n');
  process.exit(1);
}

console.log(`✅ Found ${source}\n`);

// Check if sharp is available
let sharp;
try {
  sharp = (await import('sharp')).default;
  console.log('✅ Sharp is available\n');
} catch (e) {
  console.log('❌ Sharp is not installed\n');
  console.log('To install sharp:');
  console.log('  npm install --save-dev sharp');
  console.log('  or');
  console.log('  pnpm add -D sharp\n');
  console.log('Alternative: Use online tools:');
  console.log('  - https://realfavicongenerator.net/');
  console.log('  - https://favicon.io/\n');
  process.exit(1);
}

// Generate icons
async function generateIcons() {
  try {
    const svgBuffer = readFileSync(source);
    
    // Generate favicon.ico (32x32)
    await sharp(svgBuffer)
      .resize(32, 32)
      .png()
      .toFile('favicon-32x32.png');
    console.log('✓ Generated favicon-32x32.png');
    
    // Generate icon-192.png
    await sharp(svgBuffer)
      .resize(192, 192)
      .png()
      .toFile('icon-192.png');
    console.log('✓ Generated icon-192.png');
    
    // Generate icon-512.png
    await sharp(svgBuffer)
      .resize(512, 512)
      .png()
      .toFile('icon-512.png');
    console.log('✓ Generated icon-512.png');
    
    // Generate apple-icon.png if it doesn't exist
    if (!existsSync('apple-icon.png')) {
      await sharp(svgBuffer)
        .resize(180, 180)
        .png()
        .toFile('apple-icon.png');
      console.log('✓ Generated apple-icon.png');
    } else {
      console.log('✓ apple-icon.png already exists');
    }
    
    console.log('\n✅ All icons generated successfully!');
    console.log('\n📝 Note: For favicon.ico, you can use an online converter:');
    console.log('   https://convertio.co/png-ico/');
    console.log('   Upload favicon-32x32.png and convert to favicon.ico');
    
  } catch (error) {
    console.error('\n❌ Error generating icons:', error.message);
    process.exit(1);
  }
}

generateIcons();
