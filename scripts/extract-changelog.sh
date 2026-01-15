#!/usr/bin/env bash
# ===========================================================================
# Extract Changelog for a Specific Version
# ===========================================================================
#
# This script extracts the changelog entry for a specific version from
# CHANGELOG.md (Chinese) and CHANGELOG.en.md (English).
#
# Usage:
#   ./scripts/extract-changelog.sh <version>
#
# Examples:
#   ./scripts/extract-changelog.sh v0.14.0
#   ./scripts/extract-changelog.sh 0.14.0
#
# Output:
#   Creates two files:
#   - /tmp/changelog-zh.md (Chinese changelog)
#   - /tmp/changelog-en.md (English changelog)
#
# ===========================================================================

set -euo pipefail

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
    echo "Error: Version is required"
    echo "Usage: $0 <version>"
    exit 1
fi

# Remove 'v' prefix if present
VERSION="${VERSION#v}"

# ===========================================================================
# Extract from Chinese CHANGELOG
# ===========================================================================

if [[ -f "CHANGELOG.md" ]]; then
    echo "Extracting Chinese changelog for version $VERSION..."

    # Extract content between ## [VERSION] and next ## or end of file
    awk -v ver="$VERSION" '
        /^## \[/ {
            if (found) exit
            if ($0 ~ "\\[" ver "\\]") {
                found=1
                # Skip the version header line
                next
            }
        }
        found { print }
    ' CHANGELOG.md > /tmp/changelog-zh.md

    if [[ -s /tmp/changelog-zh.md ]]; then
        echo "✓ Chinese changelog extracted to /tmp/changelog-zh.md"
    else
        echo "⚠ No Chinese changelog found for version $VERSION"
        echo "## 更新日志" > /tmp/changelog-zh.md
        echo "" >> /tmp/changelog-zh.md
        echo "请查看完整的 [CHANGELOG.md](https://github.com/wp-labs/warp-parse/blob/main/CHANGELOG.md)" >> /tmp/changelog-zh.md
    fi
else
    echo "⚠ CHANGELOG.md not found"
    echo "## 更新日志" > /tmp/changelog-zh.md
    echo "" >> /tmp/changelog-zh.md
    echo "暂无更新日志" >> /tmp/changelog-zh.md
fi

# ===========================================================================
# Extract from English CHANGELOG
# ===========================================================================

if [[ -f "CHANGELOG.en.md" ]]; then
    echo "Extracting English changelog for version $VERSION..."

    awk -v ver="$VERSION" '
        /^## \[/ {
            if (found) exit
            if ($0 ~ "\\[" ver "\\]") {
                found=1
                # Skip the version header line
                next
            }
        }
        found { print }
    ' CHANGELOG.en.md > /tmp/changelog-en.md

    if [[ -s /tmp/changelog-en.md ]]; then
        echo "✓ English changelog extracted to /tmp/changelog-en.md"
    else
        echo "⚠ No English changelog found for version $VERSION"
        echo "## Changelog" > /tmp/changelog-en.md
        echo "" >> /tmp/changelog-en.md
        echo "Please see the full [CHANGELOG.en.md](https://github.com/wp-labs/warp-parse/blob/main/CHANGELOG.en.md)" >> /tmp/changelog-en.md
    fi
else
    echo "⚠ CHANGELOG.en.md not found"
    echo "## Changelog" > /tmp/changelog-en.md
    echo "" >> /tmp/changelog-en.md
    echo "No changelog available" >> /tmp/changelog-en.md
fi

echo ""
echo "Changelog extraction complete!"
echo "Chinese: /tmp/changelog-zh.md"
echo "English: /tmp/changelog-en.md"
