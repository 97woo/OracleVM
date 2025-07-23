#!/bin/bash

# Script to fix submodule issues for team members
# This script will:
# 1. Update .gitmodules to use HTTPS URLs instead of SSH
# 2. Sync and update all submodules recursively

echo "🔧 Fixing submodule configuration..."

# Fix main submodule URL
if grep -q "git@github.com" .gitmodules; then
    echo "Converting SSH URLs to HTTPS in .gitmodules..."
    sed -i.bak 's|git@github.com:|https://github.com/|g' .gitmodules
    rm -f .gitmodules.bak
fi

# Fix nested submodule URLs
find . -name ".gitmodules" -not -path "./.git/*" | while read -r gitmodules_file; do
    if grep -q "git@github.com" "$gitmodules_file"; then
        echo "Converting SSH URLs to HTTPS in $gitmodules_file..."
        sed -i.bak 's|git@github.com:|https://github.com/|g' "$gitmodules_file"
        rm -f "${gitmodules_file}.bak"
    fi
done

# Sync all submodule URLs
echo "Syncing submodule URLs..."
git submodule sync --recursive

# Update all submodules
echo "Updating submodules..."
git submodule update --init --recursive

echo "✅ Submodules fixed successfully!"
echo ""
echo "If you still have issues, try:"
echo "1. git submodule deinit -f --all"
echo "2. rm -rf .git/modules/*"
echo "3. git submodule update --init --recursive"