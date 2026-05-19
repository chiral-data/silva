#!/bin/sh
set -e
count=$(ls inputs/ | wc -l | tr -d ' ')
echo "sequence_files=$count" > result.txt
ls inputs/ >> result.txt
# Fail if notes.txt leaked into inputs/
if [ -f inputs/notes.txt ]; then
    echo "ERROR: notes.txt should not have been copied" >&2
    exit 1
fi
