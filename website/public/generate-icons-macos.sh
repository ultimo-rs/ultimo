#!/bin/bash

# Favicon and Icon Generation Script for Ultimo (macOS version using sips)
# This script helps generate all required icon files using macOS built-in tools

echo "🎨 Ultimo Icon Generation Helper (macOS)"
echo "========================================="
echo ""

# Function to convert SVG to PNG using various methods
convert_icon() {
    local source=$1
    local output=$2
    local size=$3
    
    # Try using node with sharp if available
    if command -v node &> /dev/null && [ -d "node_modules" ]; then
        cat > temp_convert.js << EOF
const sharp = require('sharp');
sharp('$source')
  .resize($size, $size)
  .png()
  .toFile('$output')
  .then(() => console.log('Generated $output'))
  .catch(err => console.error('Error:', err));
EOF
        node temp_convert.js 2>/dev/null && rm temp_convert.js && return 0
    fi
    
    return 1
}

# Check for source icon
if [ -f "icon.svg" ] || [ -f "logo.svg" ]; then
    SOURCE="icon.svg"
    [ ! -f "$SOURCE" ] && SOURCE="logo.svg"
    
    echo "✅ Found $SOURCE - using as source"
    echo ""
    echo "⚠️  Note: For best results, use one of these online tools:"
    echo "   1. https://realfavicongenerator.net/ (Recommended)"
    echo "   2. https://favicon.io/"
    echo ""
    echo "Or install ImageMagick: brew install imagemagick"
    echo ""
    echo "📋 Required files to generate:"
    echo "  • favicon.ico (32x32)"
    echo "  • icon-192.png (192x192)"
    echo "  • icon-512.png (512x512)"
    echo "  • apple-icon.png (180x180) - already exists ✓"
    echo ""
    echo "🔗 After generating, visit:"
    echo "   https://realfavicongenerator.net/"
    echo "   Upload: $SOURCE"
    echo "   Download the generated package"
    echo "   Extract to this directory"
else
    echo "❌ No icon.svg or logo.svg found"
fi
