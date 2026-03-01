#!/bin/bash
# test_autocommit.sh
# Systemic method to verify the 1000-line Auto-Commit logic.

TEST_DIR="/tmp/matrix_overlay_test_repo"
CONFIG_FILE="$HOME/.config/matrix-overlay/config.json"

echo "--- Auto-Commit Threshold Test Preparation ---"

# 1. Create a dummy local Git repository
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"
git init
echo "Initial content" > file.txt
git add file.txt
git commit -m "Initial commit"
echo "Created test repo at $TEST_DIR"

# 2. Update config.json to include this repo and set low threshold
# We use a threshold of 5 lines for instant verification
echo "Updating config.json with test repo and threshold=5..."
python3 -c "
import json, os
path = os.path.expanduser('$CONFIG_FILE')
with open(path, 'r') as f:
    cfg = json.load(f)
cfg['productivity']['repos'] = ['$TEST_DIR']
cfg['productivity']['auto_commit_threshold'] = 5
with open(path, 'w') as f:
    json.dump(cfg, f, indent=4)
"

# 3. Simulate work (Add 10 lines)
echo "Simulating work: Adding 10 lines to file.txt..."
for i in {1..10}; do echo "Line $i" >> file.txt; done

echo "--- Instructions ---"
echo "1. Ensure matrix-overlay is running."
echo "2. Open the Settings GUI and hit 'Save & Apply' to reload the new test config."
echo "3. Wait for the hourly check (or force a reload via Tray -> Reload)."
echo "4. Check if a commit was created: cd $TEST_DIR && git log --oneline"
echo "5. To clean up, run: rm -rf $TEST_DIR"
