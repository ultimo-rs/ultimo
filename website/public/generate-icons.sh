#!/bin/bash

# Favicon and Icon Generation Script for Ultimo
# This script helps generate all required icon files

echo "🎨 Ultimo Icon Generation Helper"
echo "=================================="
echo ""

# Check if ImageMagick is installed
if command -v convert &> /dev/null; then
    echo "✅ ImageMagick is installed"
    
    # Check for source icon
    if [ -f "icon.svg" ]; then
        echo "✅ Found icon.svg - using as source"
        SOURCE="icon.svg"
        
        echo ""
        echo "Generating icons..."
        
        # Generate favicon.ico
        convert "$SOURCE" -resize 32x32 -background none -flatten favicon.ico
        echo "✓ Generated favicon.ico"
        
        # Generate icon-192.png
        convert "$SOURCE" -resize 192x192 -background none -flatten icon-192.png
        echo "✓ Generated icon-192.png"
        
        # Generate icon-512.png
        convert "$SOURCE" -resize 512x512 -background none -flatten icon-512.png
        echo "✓ Generated icon-512.png"
        
        # Check if apple-icon.png exists, if not create it
        if [ ! -f "apple-icon.png" ]; then
            convert "$SOURCE" -resize 180x180 -background none -flatten apple-icon.png
            echo "✓ Generated apple-icon.png"
        else
            echo "✓ apple-icon.png already exists"
        fi
        
        echo ""
        echo "✅ All icons generated successfully!"
        
    elif [ -f "logo.svg" ]; then
        echo "⚠️  No icon.svg found, but found logo.svg"
        echo "Using logo.svg as source..."
        SOURCE="logo.svg"
        
        echo ""
        echo "Generating icons..."
        
        convert "$SOURCE" -resize 32x32 -background none -flatten favicon.ico
        echo "✓ Generated favicon.ico"
        
        convert "$SOURCE" -resize 192x192 -background none -flatten icon-192.png
        echo "✓ Generated icon-192.png"
        
        convert "$SOURCE" -resize 512x512 -background none -flatten icon-512.png
        echo "✓ Generated icon-512.png"
        
        if [ ! -f "apple-icon.png" ]; then
            convert "$SOURCE" -resize 180x180 -background none -flatten apple-icon.png
            echo "✓ Generated apple-icon.png"
        fi
        
        echo ""
        echo "✅ All icons generated successfully!"
    else
        echo "❌ No icon.svg or logo.svg found"
        echo ""
        echo "Please provide a source icon file and run this script from the public directory"
    fi
else
    echo "❌ ImageMagick is not installed"
    echo ""
    echo "To install ImageMagick:"
    echo "  macOS:   brew install imagemagick"
    echo "  Ubuntu:  sudo apt-get install imagemagick"
    echo "  Windows: Download from https://imagemagick.org/script/download.php"
    echo ""
    echo "Alternative: Use online tools:"
    echo "  - https://realfavicongenerator.net/"
    echo "  - https://favicon.io/"
fi

echo ""
echo "📋 Required icon files:"
echo "  ✓ favicon.ico (32x32)"
echo "  ✓ icon-192.png (192x192)"
echo "  ✓ icon-512.png (512x512)"
echo "  ✓ apple-icon.png (180x180)"
