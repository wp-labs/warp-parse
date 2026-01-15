# WarpParse Documentation Index

## 📚 Official Documentation

### Release and Development Process

1. **[Release Management](RELEASE_MANAGEMENT.md)** ⭐ **PRIMARY DOCUMENT**
   - Complete release strategy and workflow
   - Daily development workflow
   - Alpha/Beta/Stable release process
   - Hotfix procedures
   - Best practices and troubleshooting

2. **[Branch Source and Maturity Control](BRANCH_SOURCE_AND_MATURITY_CONTROL.md)** 🔒 **CRITICAL POLICY**
   - Branch source control rules
   - Dependency maturity requirements
   - Decision matrix for automated handling
   - Example scenarios and troubleshooting

3. **[Branch Protection Rules](BRANCH_PROTECTION_RULES.md)** ⚙️ **SETUP GUIDE**
   - GitHub branch protection configuration
   - Immediate actions for setup
   - Long-term automation solutions

---

## 🗂️ Document Categories

### For Release Managers

- Primary: `RELEASE_MANAGEMENT.md`
- Supporting: `BRANCH_SOURCE_AND_MATURITY_CONTROL.md`
- Setup: `BRANCH_PROTECTION_RULES.md`

### For Developers

- Workflow: `RELEASE_MANAGEMENT.md` → "Daily Workflow" section
- Contributing: Follow alpha → beta → main flow

### For DevOps/SRE

- Setup: `BRANCH_PROTECTION_RULES.md`
- Automation: Check `.github/workflows/dependabot-branch-filter.yml`
- Config: Check `.github/dependabot.yml`

---

## 🔗 Quick Links

| Need | Document | Section |
|------|----------|---------|
| How to release alpha? | RELEASE_MANAGEMENT.md | 5.1 Alpha Release |
| How to release beta? | RELEASE_MANAGEMENT.md | 5.2 Beta Release |
| How to release stable? | RELEASE_MANAGEMENT.md | 5.3 Stable Release |
| Why is Dependabot PR closed? | BRANCH_SOURCE_AND_MATURITY_CONTROL.md | Enforcement Mechanisms |
| How to configure GitHub? | BRANCH_PROTECTION_RULES.md | Required Settings |
| Daily development workflow? | RELEASE_MANAGEMENT.md | 6.1 For Developers |
| Emergency hotfix? | RELEASE_MANAGEMENT.md | 5.4 Hotfix Release |

---

## 📖 Related Files

- **Workflows**: `.github/workflows/`
  - `build-and-test.yml` - CI for all branches
  - `release.yml` - Release on tag push
  - `dependabot-branch-filter.yml` - Dependency version filtering

- **Configuration**: `.github/`
  - `dependabot.yml` - Automated dependency updates

- **Scripts**: `scripts/`
  - `prepare-release.sh` - Release preparation helper

---

## 📝 Maintenance

- **Document Owner**: WarpParse Release Team
- **Review Cycle**: Quarterly or as needed
- **Last Updated**: 2026-01-15
- **Version**: 1.0

---

## 🚀 Getting Started

**New to the project?**
1. Read `RELEASE_MANAGEMENT.md` for overall strategy
2. Set up your environment following "Daily Workflow" section
3. Configure GitHub using `BRANCH_PROTECTION_RULES.md`

**Need to make a release?**
1. Follow appropriate release section in `RELEASE_MANAGEMENT.md`
2. Verify dependencies meet maturity requirements
3. Complete the release checklist

**Encountered an issue?**
1. Check "Troubleshooting" section in `RELEASE_MANAGEMENT.md`
2. Review `BRANCH_SOURCE_AND_MATURITY_CONTROL.md` for policy details
3. Check workflow logs in GitHub Actions
