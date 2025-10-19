#!/bin/bash
set -e

echo "Starting running..."
python3 -c "print('Running complete!')"

for i in 1 2 3; do
    echo "Second $i"
    sleep 3
done
