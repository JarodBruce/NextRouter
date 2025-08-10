#!/bin/bash
# This script has been refactored. Please use the new script at ./improved/nextrouter.sh

# Get the script's directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMPROVED_SCRIPT="$SCRIPT_DIR/improved/nextrouter.sh"

if [ -f "$IMPROVED_SCRIPT" ]; then
    echo "This script is deprecated. Running the new refactored script."
    echo "Please use '$IMPROVED_SCRIPT' for future executions."
    
    # Make the new script executable if it's not already
    if [ ! -x "$IMPROVED_SCRIPT" ]; then
        chmod +x "$IMPROVED_SCRIPT"
    fi
    
    # Execute the new script with all provided arguments
    "$IMPROVED_SCRIPT" "$@"
    exit 0
else
    echo "Error: The new script was not found at '$IMPROVED_SCRIPT'."
    exit 1
fi