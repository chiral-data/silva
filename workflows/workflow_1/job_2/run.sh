#!/bin/bash
set -e

echo "Starting running..."
python3 -c "print('Running complete!')"

echo "Print files from job 1"
cat hello.txt
cat bye.txt

for i in 1 2 3; do
    echo "Second $i"
    sleep 3
done
