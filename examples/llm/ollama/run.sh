#!/bin/bash
#
#

echo "ollama deepseek v1"
# ollama serve > /dev/null 2>&1 &
systemctl enable ollama
systemctl start ollama
echo "download deepseek-r1 model data"
ollama run deepseek-r1:1.5b
echo "run deepseek-r1"
echo "sleep 1 hour"
sleep 1h
