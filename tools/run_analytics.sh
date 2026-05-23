#!/bin/bash
# Compilation script for SIEM Analytics Engine (Odin)

echo "Building Odin Analytics Engine..."
# Odin requires the -file flag for single file compilation
odin build tools/analytics.odin -file -out:tools/analytics -o:speed
if [ $? -eq 0 ]; then
    echo "Successfully compiled ./tools/analytics"
    ./tools/analytics
else
    echo "Odin compiler build failed."
    exit 1
fi
