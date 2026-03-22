#!/bin/sh
set -e
# This should FAIL because 01-produce has been moved to @complete/
cp ../01-produce/outputs/result.txt stolen.txt
