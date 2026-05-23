#!/bin/bash
# Compilation script for SIEM Edge Agent (Zig Forwarder)
# Ensure Zig toolchain is installed (0.13.0+)

echo "Compiling Zig Forwarder..."
zig build-exe ../tools/forwarder.zig -O ReleaseFast -femit-bin=../tools/forwarder

if [ $? -eq 0 ]; then
    echo "Successfully compiled ./tools/forwarder"
    echo "Usage: ./tools/forwarder"
else
    echo "Compilation failed."
    exit 1
fi
