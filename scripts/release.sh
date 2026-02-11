#!/bin/bash
# Exit on errors — idempotency is handled via skip flags
set -e

# Color codes for pretty output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
VERSION=""

if [ $# -eq 0 ]; then
    echo -e "${RED}Error: Version argument required${NC}"
    echo "Usage: ./scripts/release.sh <major.minor.patch>"
    exit 1
fi

VERSION=$1

# Validate version format
if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}Error: Invalid version format. Expected: major.minor.patch (e.g., 1.2.3)${NC}"
    exit 1
fi

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Release Process for version $VERSION${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Step 1: Verify that release and tag don't already exist
echo -e "${YELLOW}[Step 1/3] Verifying release and tag status...${NC}"
echo ""

# Fetch latest tags from remote
echo "Fetching latest tags from remote..."
git fetch --tags

# Check if tag already exists
TAG="v$VERSION"
TAG_EXISTS=false
RELEASE_EXISTS=false
SKIP_VERSION_UPDATE=false
SKIP_TAG_PUSH=false

if git rev-parse "$TAG" >/dev/null 2>&1; then
    TAG_EXISTS=true
    echo -e "${YELLOW}⚠️  Warning: Tag $TAG already exists${NC}"
    
    # Check commit the tag points to
    TAG_COMMIT=$(git rev-parse "$TAG")
    echo "   Tag points to commit: $TAG_COMMIT"
    SKIP_TAG_PUSH=true
else
    echo -e "${GREEN}✅ Tag $TAG does not exist${NC}"
fi

# Check if release exists on GitHub
echo "Checking if GitHub release exists..."

# Extract repository info from git remote
GIT_REMOTE=$(git remote get-url origin)
if [[ $GIT_REMOTE =~ github\.com[:/]([^/]+)/([^/.]+)(\.git)?$ ]]; then
    REPO_OWNER="${BASH_REMATCH[1]}"
    REPO_NAME="${BASH_REMATCH[2]}"
    
    # Check if release exists via GitHub API
    HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "https://api.github.com/repos/$REPO_OWNER/$REPO_NAME/releases/tags/$TAG")
    
    if [ "$HTTP_STATUS" = "200" ]; then
        RELEASE_EXISTS=true
        echo -e "${YELLOW}⚠️  Warning: GitHub release $TAG already exists${NC}"
        echo "   You can view it at: https://github.com/$REPO_OWNER/$REPO_NAME/releases/tag/$TAG"
        SKIP_VERSION_UPDATE=true
        SKIP_TAG_PUSH=true
    elif [ "$HTTP_STATUS" = "404" ]; then
        echo -e "${GREEN}✅ No existing release found${NC}"
    else
        echo -e "${RED}❌  Error: Could not verify release status (HTTP $HTTP_STATUS)${NC}"
        exit 1
    fi
else
    echo -e "${RED}❌  Error: Could not parse GitHub repository from remote${NC}"
    exit 1
fi

echo ""

# Step 2: Update version files, create branch, push, and wait for PR merge
echo -e "${YELLOW}[Step 2/3] Updating version and creating PR...${NC}"
echo ""

if [ "$SKIP_VERSION_UPDATE" = true ]; then
    echo -e "${YELLOW}⚠️  Skipping version update (release already exists)${NC}"
    echo ""
else
    # Call update-version.sh
    ./scripts/update-version.sh "$VERSION"
    
    echo ""
    echo -e "${GREEN}✅ Version update complete${NC}"
fi

echo ""

# Step 3: Push tag and monitor release
echo -e "${YELLOW}[Step 3/3] Creating tag and monitoring release...${NC}"
echo ""

# Ensure we are on master before tagging
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$CURRENT_BRANCH" != "master" ]; then
    echo "Switching to master branch..."
    git checkout master
    git pull origin master
    echo -e "${GREEN}✅ On master branch${NC}"
    echo ""
fi

# Final version verification before tagging
if [ "$SKIP_TAG_PUSH" != true ]; then
    source "$(dirname "$0")/check-version.sh" "$VERSION"
    if [ "$HAS_MISMATCH" = true ]; then
        echo -e "${RED}❌  Error: Version mismatch detected before tagging!${NC}"
        echo -e "${RED}   Files must match version $VERSION before creating a tag.${NC}"
        echo -e "${RED}   Run: ./scripts/update-version.sh --files-only $VERSION${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ All file versions match $VERSION${NC}"
    echo ""
fi

if [ "$SKIP_TAG_PUSH" = true ]; then
    echo -e "${YELLOW}⚠️  Skipping tag push (tag already exists)${NC}"
    
    if [ "$RELEASE_EXISTS" = true ]; then
        echo -e "${GREEN}✅ Release already exists and is available!${NC}"
        echo "   View at: https://github.com/$REPO_OWNER/$REPO_NAME/releases/tag/$TAG"
    else
        echo "Monitoring for release creation..."
        # Run push-tag.sh to monitor the release
        ./scripts/push-tag.sh "$VERSION"
    fi
else
    # Run push-tag.sh to create a new tag and monitor the release
    ./scripts/push-tag.sh "$VERSION"
fi

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}✅ Release process complete!${NC}"
echo -e "${GREEN}========================================${NC}"
