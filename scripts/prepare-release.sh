#!/usr/bin/env bash
# ===========================================================================
# Release Preparation Script
# ===========================================================================
#
# This script helps prepare releases by checking dependency versions
# and ensuring they match the target maturity level.
#
# Usage:
#   ./scripts/prepare-release.sh <version> <maturity>
#
# Examples:
#   ./scripts/prepare-release.sh 0.14.0 alpha
#   ./scripts/prepare-release.sh 0.14.0 beta
#   ./scripts/prepare-release.sh 0.14.0 stable
#
# ===========================================================================

set -euo pipefail

VERSION="${1:-}"
MATURITY="${2:-}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ===========================================================================
# Helper Functions
# ===========================================================================

error() {
    echo -e "${RED}✗ Error: $1${NC}" >&2
    exit 1
}

success() {
    echo -e "${GREEN}✓ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠ Warning: $1${NC}"
}

info() {
    echo -e "${BLUE}ℹ Info: $1${NC}"
}

# ===========================================================================
# Validation
# ===========================================================================

if [[ -z "$VERSION" ]]; then
    error "Version is required. Usage: $0 <version> <maturity>"
fi

if [[ -z "$MATURITY" ]]; then
    error "Maturity is required. Usage: $0 <version> <maturity>"
fi

if [[ ! "$MATURITY" =~ ^(alpha|beta|stable)$ ]]; then
    error "Maturity must be one of: alpha, beta, stable"
fi

# ===========================================================================
# Check Dependencies
# ===========================================================================

info "Checking dependencies for ${MATURITY} release v${VERSION}..."
echo

# Check Git dependencies
check_git_dependency() {
    local dep_name="$1"
    local pattern="$2"

    # Extract tag from Cargo.toml
    local tag=$(grep -A 1 "package = \"$dep_name\"" Cargo.toml | grep "tag =" | sed -E 's/.*tag = "([^"]+)".*/\1/')

    if [[ -z "$tag" ]]; then
        warning "Could not find $dep_name in Cargo.toml"
        return
    fi

    echo "  $dep_name: $tag"

    case "$MATURITY" in
        alpha)
            # Alpha can use any version
            success "    ✓ Any version acceptable for alpha"
            ;;
        beta)
            # Beta should use beta or stable versions
            if [[ "$tag" =~ -alpha ]]; then
                error "    ✗ Beta release should not use alpha dependencies: $tag"
            else
                success "    ✓ Version acceptable for beta"
            fi
            ;;
        stable)
            # Stable should only use stable versions
            if [[ "$tag" =~ -alpha|-beta ]]; then
                error "    ✗ Stable release should only use stable dependencies: $tag"
            else
                success "    ✓ Version acceptable for stable"
            fi
            ;;
    esac
}

echo "Git Dependencies:"
check_git_dependency "wp-engine" "wp-engine"
check_git_dependency "wp-config" "wp-config"
check_git_dependency "wp-lang" "wp-lang"
check_git_dependency "wp-knowledge" "wp-knowledge"
check_git_dependency "wp-cli-core" "wp-cli-core"
check_git_dependency "wp-proj" "wp-proj"
check_git_dependency "wp-connectors" "wp-connectors"

echo

# ===========================================================================
# Update Development Stage
# ===========================================================================

info "Updating development stage to ${MATURITY}..."

if [[ -f ".dev-stage.yml" ]]; then
    sed -i.bak "s/^stage: .*/stage: $MATURITY/" .dev-stage.yml
    sed -i.bak "s/^next_version: .*/next_version: $VERSION/" .dev-stage.yml
    rm -f .dev-stage.yml.bak
    success "Updated .dev-stage.yml"
else
    warning ".dev-stage.yml not found, skipping"
fi

echo

# ===========================================================================
# Check Version in Cargo.toml
# ===========================================================================

info "Checking Cargo.toml version..."
CARGO_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed -E 's/version = "([^"]+)"/\1/')

EXPECTED_VERSION="$VERSION"

if [[ "$CARGO_VERSION" != "$EXPECTED_VERSION" ]]; then
    warning "Cargo.toml version ($CARGO_VERSION) does not match expected version ($EXPECTED_VERSION)"
    warning "Please update Cargo.toml manually or run:"
    echo "  sed -i '' 's/^version = \".*\"/version = \"$EXPECTED_VERSION\"/' Cargo.toml"
else
    success "Cargo.toml version matches: $CARGO_VERSION"
fi

echo

# ===========================================================================
# Run Tests
# ===========================================================================

info "Running tests..."
if cargo test --quiet; then
    success "All tests passed"
else
    error "Tests failed. Please fix before releasing."
fi

echo

# ===========================================================================
# Suggest Next Steps
# ===========================================================================

case "$MATURITY" in
    alpha)
        TAG_NAME="v${VERSION}-alpha.1"
        ;;
    beta)
        TAG_NAME="v${VERSION}-beta.1"
        ;;
    stable)
        TAG_NAME="v${VERSION}"
        ;;
esac

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Ready to create ${MATURITY} release!${NC}"
echo -e "${GREEN}========================================${NC}"
echo
echo "Next steps:"
echo "  1. Review changes:"
echo "     git status"
echo "  2. Review CHANGELOG.md"
echo "  3. Commit all changes (including .dev-stage.yml):"
echo "     git add -A"
echo "     git commit -m 'chore: prepare release $TAG_NAME'"
echo "  4. Create and push tag:"
echo "     git tag $TAG_NAME"
echo "     git push origin main"
echo "     git push origin $TAG_NAME"
echo "  5. Wait for CI/CD to complete"
echo "  6. Verify release at: https://github.com/wp-labs/warp-parse/releases/tag/$TAG_NAME"
echo
echo "📝 Note: .dev-stage.yml has been updated to '$MATURITY' stage."
echo "   Dependabot will now automatically handle dependency updates according to this stage."
echo
