#!/bin/bash

echo "🚀 rou2exOS Build and Run Script"
echo "================================"

# 1. Build Docker image
echo "📦 Building Docker image..."
docker build -t rou2exos-builder .

if [ $? -ne 0 ]; then
    echo "❌ Docker build failed"
    exit 1
fi

# 2. Run container and copy ISO
echo "📋 Copying ISO file..."
docker run --rm -v $(pwd):/host rou2exos-builder bash -c "
    if [ -f r2.iso ]; then
        cp r2.iso /host/
        echo '✅ ISO file copied successfully: r2.iso'
        exit
    else
        echo '❌ ISO file not found'
        ls -la
        exit 1
    fi
"

if [ $? -ne 0 ]; then
    echo "❌ ISO copy failed"
    exit 1
fi

# 3. Verify ISO file exists
if [ ! -f "r2.iso" ]; then
    echo "❌ r2.iso file not found"
    exit 1
fi

# 4. Run QEMU
echo "🖥️  Running rou2exOS with QEMU..."
echo "To exit: Press Ctrl+Alt+G, then type 'quit' in QEMU monitor"
qemu-system-x86_64 -boot d -cdrom r2.iso

echo "✅ Done!"