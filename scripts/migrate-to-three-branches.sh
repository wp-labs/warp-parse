#!/usr/bin/env bash
# ===========================================================================
# Migration Script: Single Branch → Three Branches
# ===========================================================================
#
# This script helps migrate from single-branch+tags strategy to three-branch
# strategy (alpha/beta/main).
#
# Usage:
#   ./scripts/migrate-to-three-branches.sh
#
# ===========================================================================

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Migrate to Three-Branch Strategy${NC}"
echo -e "${BLUE}========================================${NC}"
echo

# ===========================================================================
# Pre-flight Checks
# ===========================================================================

info "Running pre-flight checks..."

# Check if on main branch
CURRENT_BRANCH=$(git branch --show-current)
if [[ "$CURRENT_BRANCH" != "main" ]]; then
    warning "Not on main branch (currently on: $CURRENT_BRANCH)"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        error "Aborted"
    fi
fi

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    error "You have uncommitted changes. Please commit or stash them first."
fi

# Check if remote exists
if ! git remote get-url origin &>/dev/null; then
    error "No remote 'origin' found"
fi

success "Pre-flight checks passed"
echo

# ===========================================================================
# Create Branches
# ===========================================================================

info "Creating alpha and beta branches from main..."

# Create alpha branch
if git show-ref --verify --quiet refs/heads/alpha; then
    warning "Branch 'alpha' already exists, skipping creation"
else
    git checkout -b alpha
    success "Created branch 'alpha'"
fi

# Create beta branch
git checkout main
if git show-ref --verify --quiet refs/heads/beta; then
    warning "Branch 'beta' already exists, skipping creation"
else
    git checkout -b beta
    success "Created branch 'beta'"
fi

# Return to main
git checkout main

echo

# ===========================================================================
# Update Files
# ===========================================================================

info "Updating configuration files..."

# Backup old files
mkdir -p .migration-backup
if [[ -f ".dev-stage.yml" ]]; then
    cp .dev-stage.yml .migration-backup/
    success "Backed up .dev-stage.yml"
fi

if [[ -f ".github/workflows/dependabot-auto-merge.yml" ]]; then
    cp .github/workflows/dependabot-auto-merge.yml .migration-backup/
    success "Backed up dependabot-auto-merge.yml"
fi

# Switch to alpha branch for updates
git checkout alpha

# Replace dependabot.yml
if [[ -f ".github/dependabot-three-branch.yml" ]]; then
    cp .github/dependabot-three-branch.yml .github/dependabot.yml
    success "Updated .github/dependabot.yml"
else
    warning ".github/dependabot-three-branch.yml not found, skipping"
fi

# Remove old single-branch files
if [[ -f ".dev-stage.yml" ]]; then
    git rm .dev-stage.yml
    success "Removed .dev-stage.yml"
fi

if [[ -f ".github/workflows/dependabot-auto-merge.yml" ]]; then
    git rm .github/workflows/dependabot-auto-merge.yml
    success "Removed .github/workflows/dependabot-auto-merge.yml"
fi

if [[ -f "docs/AUTOMATED_DEPENDENCY_MANAGEMENT.md" ]]; then
    git rm docs/AUTOMATED_DEPENDENCY_MANAGEMENT.md
    success "Removed docs/AUTOMATED_DEPENDENCY_MANAGEMENT.md"
fi

# Remove temporary three-branch config file
if [[ -f ".github/dependabot-three-branch.yml" ]]; then
    git rm .github/dependabot-three-branch.yml
    success "Removed .github/dependabot-three-branch.yml"
fi

# Commit changes
if ! git diff-index --quiet HEAD --; then
    git add -A
    git commit -m "chore: migrate to three-branch strategy

- Replace single-branch dependabot config with three-branch config
- Remove .dev-stage.yml (no longer needed)
- Remove dependabot-auto-merge.yml (replaced with branch-filter)
- Add dependabot-branch-filter.yml for simple version filtering

See docs/STRATEGY_COMPARISON.md for details."
    success "Committed migration changes to alpha branch"
else
    info "No changes to commit"
fi

echo

# ===========================================================================
# Push Branches
# ===========================================================================

info "Pushing branches to remote..."

read -p "Push alpha, beta, and main branches to origin? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    git push origin alpha
    git push origin beta
    git push origin main
    success "Pushed all branches to origin"
else
    warning "Skipped pushing to remote"
fi

echo

# ===========================================================================
# Instructions
# ===========================================================================

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Migration Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo
echo "Next steps:"
echo
echo "1. Set default branch on GitHub:"
echo "   → Go to: https://github.com/YOUR-ORG/warp-parse/settings/branches"
echo "   → Change default branch from 'main' to 'alpha'"
echo
echo "2. Review the new branch structure:"
echo "   git branch -a"
echo
echo "3. Verify Dependabot configuration:"
echo "   cat .github/dependabot.yml"
echo
echo "4. Read the strategy comparison:"
echo "   cat docs/STRATEGY_COMPARISON.md"
echo
echo "5. Start working on alpha branch:"
echo "   git checkout alpha"
echo
echo "Branch roles:"
echo "  - alpha: Development (accepts all dependency versions)"
echo "  - beta:  Testing (accepts beta and stable versions)"
echo "  - main:  Production (accepts stable versions only)"
echo
echo "Workflow:"
echo "  alpha → beta → main"
echo "  (merge forward when ready for next maturity level)"
echo
echo -e "${BLUE}Backup files saved in: .migration-backup/${NC}"
echo
