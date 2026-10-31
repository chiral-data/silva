#!/bin/bash
echo "cpu.max: $(cat /sys/fs/cgroup/cpu.max)"
echo "memory.max: $(cat /sys/fs/cgroup/memory.max)"
# Absent on a host without swap accounting, where silva only logs the dropped cap.
if [ -f /sys/fs/cgroup/memory.swap.max ]; then
    echo "memory.swap.max: $(cat /sys/fs/cgroup/memory.swap.max)"
fi
