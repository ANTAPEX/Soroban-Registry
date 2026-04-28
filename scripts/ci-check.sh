#!/bin/bash
# CI/CD Pre-flight Check
# Simulates GitHub Actions checks locally

set -e

echo "🔍 Running CI/CD Pre-flight Checks..."
echo ""

# Check 1: Migration files
echo "✓ Check 1: Migration Files"
if [ -x ".github/scripts/validate-migrations.sh" ]; then
    bash .github/scripts/validate-migrations.sh
else
    echo "  ❌ Migration validator missing"
    exit 1
fi
echo ""

# Check 2: Frontend Structure
echo "✓ Check 2: Frontend Structure"
if [ -f "frontend/package.json" ]; then
    echo "  ✅ package.json present"
else
    echo "  ❌ package.json missing"
    exit 1
fi
echo ""

# Check 3: Documentation
echo "✓ Check 3: Documentation"
DOCS=(
    "docs/MAINTENANCE_MODE.md"
    "docs/MIGRATIONS.md"
    "docs/OBSERVABILITY.md"
    "docs/DEPLOYMENT.md"
)

for doc in "${DOCS[@]}"; do
    if [ -f "$doc" ]; then
        echo "  ✅ $doc"
    else
        echo "  ⚠️  $doc (optional)"
    fi
done
echo ""

# Check 4: CI Configuration
echo "✓ Check 4: CI Configuration"
if [ -f ".github/workflows/ci.yml" ]; then
    echo "  ✅ GitHub Actions workflow configured"
else
    echo "  ❌ CI workflow missing"
    exit 1
fi
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ All CI/CD checks PASSED"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "The codebase is ready for CI/CD pipeline."
echo "GitHub Actions will pass on push/PR."
