#!/bin/sh
set -e
# Proper usage: read from inputs/ directory
cat inputs/data.csv > report.txt
cat inputs/metadata.json >> report.txt
echo "report complete" >> report.txt
