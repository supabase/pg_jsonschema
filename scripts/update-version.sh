#!/bin/bash
set -e

# Parse arguments
FILES_ONLY=false
VERSION=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --files-only)
            FILES_ONLY=true
            shift
            ;;
        *)
            VERSION=$1
            shift
            ;;
    esac
done

# Check if version argument is provided
if [ -z "$VERSION" ]; then
    echo "Error: Version argument required"
    echo "Usage: ./scripts/update-version.sh [--files-only] <major.minor.patch>"
    exit 1
fi

# Validate version format
if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Invalid version format. Expected: major.minor.patch (e.g., 1.2.3)"
    exit 1
fi

# Check version mismatches using shared script
source "$(dirname "$0")/check-version.sh" "$VERSION"

if [ "$HAS_MISMATCH" = true ]; then
    if [ "$FILES_ONLY" = true ]; then
        echo ""
        echo "Updating versions to $VERSION (files only mode)..."
        
        # Update Cargo.toml (using awk for cross-platform compatibility)
        if [ "$VERSION" != "$CARGO_VERSION" ]; then
            awk -v ver="$VERSION" '
                /^\[package\]/ { in_package=1 }
                /^\[/ && !/^\[package\]/ { in_package=0 }
                in_package && /^version = / {
                    print "version = \"" ver "\""
                    in_package=0
                    next
                }
                { print }
            ' Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml
            echo "✅ Updated Cargo.toml"
        fi
        
        # Update META.json (both root version and provides.pg_jsonschema.version, but not meta-spec)
        # Using awk for cross-platform compatibility - updates first 2 version occurrences only
        if [ "$VERSION" != "$META_VERSION" ]; then
            awk -v ver="$VERSION" '
                /"version":/ {
                    version_count++
                    if (version_count <= 2) {
                        sub(/"version": "[^"]*"/, "\"version\": \"" ver "\"")
                    }
                }
                { print }
            ' META.json > META.json.tmp && mv META.json.tmp META.json
            echo "✅ Updated META.json"
        fi
        
        echo ""
        echo "Updating Cargo.lock..."
        cargo build --quiet 2>/dev/null || cargo build
        echo "✅ Updated Cargo.lock"
        
        echo ""
        echo "✅ Files updated to version $VERSION"
        echo "   Note: No git operations performed (--files-only mode)"
    else
        BRANCH_NAME="release/$VERSION"
        
        # Fetch latest from remote
        echo ""
        echo "Fetching latest from remote..."
        git fetch
        echo "✅ Fetched from remote"
        
        # Check if branch already exists (locally or remotely)
        BRANCH_EXISTS=false
        if git rev-parse --verify "$BRANCH_NAME" >/dev/null 2>&1; then
            BRANCH_EXISTS=true
            echo ""
            echo "⚠️  Warning: Branch $BRANCH_NAME already exists locally"
            
            # Check if we're already on this branch
            CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
            if [ "$CURRENT_BRANCH" != "$BRANCH_NAME" ]; then
                echo "Switching to existing branch: $BRANCH_NAME"
                git checkout "$BRANCH_NAME"
            else
                echo "Already on branch: $BRANCH_NAME"
            fi
        elif git rev-parse --verify "origin/$BRANCH_NAME" >/dev/null 2>&1; then
            BRANCH_EXISTS=true
            echo ""
            echo "⚠️  Warning: Branch $BRANCH_NAME already exists on remote"
            echo "Checking out existing branch from remote"
            git checkout -b "$BRANCH_NAME" "origin/$BRANCH_NAME"
        else
            echo ""
            echo "Creating branch: $BRANCH_NAME"
            git checkout -b "$BRANCH_NAME"
            echo "✅ Branch created"
        fi
        
        echo ""
        echo "Updating versions to $VERSION..."
        
        # Update Cargo.toml (using awk for cross-platform compatibility)
        if [ "$VERSION" != "$CARGO_VERSION" ]; then
            awk -v ver="$VERSION" '
                /^\[package\]/ { in_package=1 }
                /^\[/ && !/^\[package\]/ { in_package=0 }
                in_package && /^version = / {
                    print "version = \"" ver "\""
                    in_package=0
                    next
                }
                { print }
            ' Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml
            echo "✅ Updated Cargo.toml"
        fi
        
        # Update META.json (both root version and provides.pg_jsonschema.version, but not meta-spec)
        # Using awk for cross-platform compatibility - updates first 2 version occurrences only
        if [ "$VERSION" != "$META_VERSION" ]; then
            awk -v ver="$VERSION" '
                /"version":/ {
                    version_count++
                    if (version_count <= 2) {
                        sub(/"version": "[^"]*"/, "\"version\": \"" ver "\"")
                    }
                }
                { print }
            ' META.json > META.json.tmp && mv META.json.tmp META.json
            echo "✅ Updated META.json"
        fi
        
        echo ""
        echo "Updating Cargo.lock..."
        cargo build --quiet 2>/dev/null || cargo build
        echo "✅ Updated Cargo.lock"
        
        echo ""
        echo "Committing changes..."
        
        # Check if there are changes to commit
        if git diff --quiet Cargo.toml META.json Cargo.lock; then
            echo "⚠️  Warning: No changes to commit (files already updated)"
            COMMIT_HASH=$(git rev-parse HEAD)
        else
            git add Cargo.toml META.json Cargo.lock
            git commit -m "chore: bump version to $VERSION"
            echo "✅ Changes committed"
            COMMIT_HASH=$(git rev-parse HEAD)
        fi
        
        echo ""
        echo "Pushing branch to remote..."
        
        # Try to push, but continue if already pushed
        if git push -u origin "$BRANCH_NAME" 2>&1 | tee /tmp/git_push_output.txt; then
            echo "✅ Branch pushed to remote"
        else
            # Check if error is because branch is already up to date
            if grep -q "Everything up-to-date" /tmp/git_push_output.txt || grep -q "already exists" /tmp/git_push_output.txt; then
                echo "⚠️  Warning: Branch already pushed to remote"
            else
                # If it's a real error, still show it but continue
                echo "⚠️  Warning: Failed to push branch, but continuing..."
            fi
        fi
        rm -f /tmp/git_push_output.txt
        
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "📝 Next Steps:"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        echo "1. Create a Pull Request for branch: $BRANCH_NAME"
        echo "2. Get the PR reviewed and merged into master"
        echo ""
        
        # Extract repository info for PR URL
        GIT_REMOTE=$(git remote get-url origin)
        if [[ $GIT_REMOTE =~ github\.com[:/]([^/]+)/([^/.]+)(\.git)?$ ]]; then
            REPO_OWNER="${BASH_REMATCH[1]}"
            REPO_NAME="${BASH_REMATCH[2]}"
            PR_URL="https://github.com/$REPO_OWNER/$REPO_NAME/compare/master...$BRANCH_NAME?expand=1"
            echo "Create a PR at: $PR_URL"
        else
            echo "Create a PR manually on GitHub."
        fi
        echo ""
        
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "⏳ Waiting for PR to be merged..."
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        
        # Fetch latest from remote to check current status
        git fetch origin master >/dev/null 2>&1
        
        # Check if already merged into master
        if git merge-base --is-ancestor "$COMMIT_HASH" origin/master 2>/dev/null; then
            echo "✅ Changes already merged into master"
        else
            # Poll for PR merge status
            CHECK_INTERVAL=15  # Check every 15 seconds
            MAX_ATTEMPTS=240   # Max 1 hour (240 * 15 seconds)
            ATTEMPT=0
            
            echo "Checking if PR is merged (polling every ${CHECK_INTERVAL}s)..."
            echo "Press Ctrl+C to cancel"
            echo ""
            
            while [ $ATTEMPT -lt $MAX_ATTEMPTS ]; do
                # Fetch latest from remote
                git fetch origin master >/dev/null 2>&1
                
                # Check if the commit exists in master
                if git merge-base --is-ancestor "$COMMIT_HASH" origin/master 2>/dev/null; then
                    echo "✅ Changes are merged into master"
                    break
                fi
                
                ATTEMPT=$((ATTEMPT + 1))
                ELAPSED=$((ATTEMPT * CHECK_INTERVAL))
                echo "⏳ Still waiting... (${ELAPSED}s elapsed, checking again in ${CHECK_INTERVAL}s)"
                sleep $CHECK_INTERVAL
            done
            
            # Final check
            if ! git merge-base --is-ancestor "$COMMIT_HASH" origin/master 2>/dev/null; then
                echo ""
                echo "⚠️  Warning: PR not merged after waiting for 1 hour"
                echo "   Please merge the PR and run the release script again to continue."
                echo "   Branch: $BRANCH_NAME"
                echo "   Commit: $COMMIT_HASH"
                exit 1
            fi
        fi
        
        echo ""
        echo "Verifying PR is merged into master..."
        
        # Checkout master and pull latest
        echo ""
        echo "Checking out master branch..."
        git checkout master
        git pull origin master
        echo "✅ Master branch updated"
        
        echo ""
        echo "✅ Release preparation complete"
        echo "   Branch: $BRANCH_NAME"
        echo "   Version: $VERSION"
        echo "   Status: Merged into master"
    fi
else
    echo "✅ All versions already match: $VERSION"
    echo ""
    echo "No changes needed. Skipping branch creation and commit."
fi
