#!/bin/bash
# setup_rust.sh - Complete Rust setup for C2 agent generation

echo "=== Setting up Rust for C2 Agent Generation ==="

# Remove any existing rustup installation
echo "Cleaning up existing Rust installation..."
rm -rf ~/.cargo ~/.rustup

# Install rustup
echo "Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable

# Source the environment
source ~/.cargo/env

# Verify installation
echo "Verifying Rust installation..."
rustc --version
cargo --version

# Set default toolchain explicitly
echo "Setting default toolchain..."
rustup default stable
rustup update

# Install Windows cross-compilation target
echo "Installing Windows cross-compilation tools..."
rustup target add x86_64-pc-windows-gnu

# Install mingw for cross-compilation
echo "Installing MinGW cross-compiler..."
sudo apt update
sudo apt install -y gcc-mingw-w64-x86-64 gcc-mingw-w64-i686

# Create cargo config for cross-compilation
mkdir -p ~/.cargo
cat > ~/.cargo/config.toml << 'EOF'
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"

[target.i686-pc-windows-gnu]
linker = "i686-w64-mingw32-gcc"
EOF

echo "=== Rust setup complete! ==="
echo "You can now generate Windows agents."