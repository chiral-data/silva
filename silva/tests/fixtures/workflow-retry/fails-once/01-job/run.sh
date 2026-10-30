#!/bin/bash
# The job folder survives between attempts, so the count does too.
n=$(cat attempts 2>/dev/null || echo 0)
n=$((n + 1))
echo "$n" > attempts
echo "attempt $n"
[ "$n" -ge 2 ]
