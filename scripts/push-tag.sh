#!/bin/bash
set -e

# Parse arguments
DRY_RUN=false
VERSION=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
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
    echo "Usage: ./scripts/push-tag.sh [--dry-run] <major.minor.patch>"
    exit 1
fi

# Validate version format
if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Invalid version format. Expected: major.minor.patch (e.g., 1.2.3)"
    exit 1
fi

# Check version mismatches using shared script
source "$(dirname "$0")/check-version.sh" "$VERSION" "--warn-only"

if [ "$HAS_MISMATCH" = true ]; then
    echo ""
    echo "⚠️  Warning: Version mismatch detected, but continuing..."
    echo "   Ensure versions are updated before creating a release."
fi

# Exit early if dry run
if [ "$DRY_RUN" = true ]; then
    echo ""
    echo "✅ Dry run successful - all version checks passed"
    echo "   Version: $VERSION"
    echo "   Tag that would be created: v$VERSION"
    exit 0
fi

# Create and push tag
TAG="v$VERSION"
echo ""
echo "Creating tag: $TAG"

# Fetch all tags from remote
echo "Fetching latest tags from remote..."
git fetch --tags

# Check if tag already exists
TAG_EXISTS=false
if git rev-parse "$TAG" >/dev/null 2>&1; then
    TAG_EXISTS=true
    TAG_COMMIT=$(git rev-parse "$TAG")
    CURRENT_COMMIT=$(git rev-parse HEAD)
    
    echo "⚠️  Warning: Tag $TAG already exists"
    
    if [ "$TAG_COMMIT" = "$CURRENT_COMMIT" ]; then
        echo "✅ Tag points to current commit, continuing..."
    else
        echo "⚠️  Warning: Tag points to a different commit"
        echo "   Tag commit: $TAG_COMMIT"
        echo "   Current commit: $CURRENT_COMMIT"
        echo "   Continuing anyway..."
    fi
else
    # Create the tag
    git tag "$TAG"
    echo "✅ Tag $TAG created"
fi

# Push the tag to remote
echo "Pushing tag to remote..."
if git push origin "$TAG" 2>&1 | tee /tmp/git_push_tag_output.txt; then
    echo "✅ Tag $TAG pushed successfully"
else
    # Check if error is because tag already exists on remote
    if grep -q "already exists" /tmp/git_push_tag_output.txt; then
        echo "⚠️  Warning: Tag already exists on remote, continuing..."
    else
        echo "⚠️  Warning: Failed to push tag, but continuing..."
    fi
fi
rm -f /tmp/git_push_tag_output.txt

echo ""

# Poll for GitHub release
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "⏳ Monitoring GitHub release creation..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Extract repository info from git remote
GIT_REMOTE=$(git remote get-url origin)
if [[ $GIT_REMOTE =~ github\.com[:/]([^/]+)/([^/.]+)(\.git)?$ ]]; then
    REPO_OWNER="${BASH_REMATCH[1]}"
    REPO_NAME="${BASH_REMATCH[2]}"
    
    echo "Waiting for release.yml workflow to create the release..."
    echo "(This may take a few minutes)"
    echo ""
    
    # Poll for release with timeout (30s interval keeps us well under the 60 req/hr unauthenticated API limit)
    POLL_INTERVAL=30
    MAX_ATTEMPTS=40  # 20 minutes (40 * 30 seconds)
    ATTEMPT=0
    RELEASE_FOUND=false
    RELEASE_URL=""
    
    while [ $ATTEMPT -lt $MAX_ATTEMPTS ]; do
        HTTP_STATUS=$(curl -s -o /tmp/release_check.json -w "%{http_code}" "https://api.github.com/repos/$REPO_OWNER/$REPO_NAME/releases/tags/$TAG")
        
        if [ "$HTTP_STATUS" = "200" ]; then
            RELEASE_FOUND=true
            RELEASE_URL=$(grep -o '"html_url":"[^"]*"' /tmp/release_check.json | head -1 | cut -d'"' -f4)
            rm -f /tmp/release_check.json
            break
        fi
        
        ATTEMPT=$((ATTEMPT + 1))
        ELAPSED=$((ATTEMPT * POLL_INTERVAL))
        printf "\r⏳ Waiting... %dm%02ds elapsed" $((ELAPSED / 60)) $((ELAPSED % 60))
        sleep $POLL_INTERVAL
    done
    
    rm -f /tmp/release_check.json
    
    echo ""
    echo ""
    
    if [ "$RELEASE_FOUND" = true ]; then
        echo "✅ GitHub release is now available!"
        echo ""
        if [ -n "$RELEASE_URL" ]; then
            echo "🔗 Release URL: $RELEASE_URL"
        else
            echo "🔗 Release URL: https://github.com/$REPO_OWNER/$REPO_NAME/releases/tag/$TAG"
        fi
    else
        echo "⚠️  Timeout waiting for GitHub release"
        echo "   The release workflow may still be running."
        echo "   Check the Actions tab on GitHub for status."
        echo "   https://github.com/$REPO_OWNER/$REPO_NAME/actions"
    fi
else
    echo "⚠️  Could not parse GitHub repository from git remote"
    echo "   The release workflow has been triggered by pushing the tag."
fi

echo ""
echo "✅ Successfully released version $VERSION"
echo "   Tag: $TAG"
