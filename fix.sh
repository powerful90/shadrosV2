#!/bin/bash
# NUCLEAR FIX - Complete Rust environment reset

echo "🚨 NUCLEAR FIX: Complete Rust environment reset"
echo "==============================================="

# Check what's in the corrupted eframe lib.rs
echo "1. Checking corrupted eframe lib.rs..."
EFRAME_PATH="/home/powerful/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.23.0/src/lib.rs"
if [ -f "$EFRAME_PATH" ]; then
    echo "❌ Found corrupted eframe lib.rs:"
    head -10 "$EFRAME_PATH"
    echo ""
fi

# Step 1: Complete cargo directory removal
echo "2. Removing entire cargo directory..."
rm -rf ~/.cargo/

# Step 2: Recreate cargo directory
echo "3. Recreating cargo directory..."
mkdir -p ~/.cargo/bin

# Step 3: Reinstall cargo components if needed
echo "4. Updating rustup..."
rustup self update || echo "Rustup update failed - continuing..."
rustup update || echo "Rustup toolchain update failed - continuing..."

# Step 4: Clear all project artifacts
echo "5. Cleaning project thoroughly..."
cargo clean || echo "Cargo clean failed - continuing..."
rm -rf target/
rm -rf Cargo.lock

# Step 5: Check for hidden corrupted files
echo "6. Checking for hidden corrupted files..."
find . -name ".cargo" -type d 2>/dev/null && rm -rf ./.cargo
find . -name "*.rlib" -type f 2>/dev/null | xargs rm -f
find . -name "*.rmeta" -type f 2>/dev/null | xargs rm -f

# Step 6: Verify project structure
echo "7. Verifying project structure..."
echo "Current directory: $(pwd)"
echo "Project files:"
ls -la

echo "Source files:"
if [ -d "src" ]; then
    ls -la src/
else
    echo "❌ No src/ directory found!"
fi

# Step 7: Check for lib.rs (should not exist)
echo "8. Checking for unwanted lib.rs files..."
find . -name "lib.rs" -type f | while read file; do
    echo "❌ Found unwanted lib.rs: $file"
    echo "Contents:"
    head -5 "$file"
    echo "Removing..."
    rm -f "$file"
done

# Step 8: Check Cargo.toml for [lib] sections
echo "9. Checking Cargo.toml for library configurations..."
if [ -f "Cargo.toml" ]; then
    if grep -q "\[lib\]" Cargo.toml; then
        echo "❌ Found [lib] section in Cargo.toml - this should not exist for a binary project!"
        echo "Please remove the [lib] section from Cargo.toml"
    else
        echo "✅ No [lib] section found in Cargo.toml"
    fi
else
    echo "❌ No Cargo.toml found!"
fi

echo ""
echo "✅ Nuclear cleanup complete!"
echo ""
echo "🔧 Next steps:"
echo "   1. Verify your Cargo.toml has no [lib] section"
echo "   2. Ensure no lib.rs files exist in your project"
echo "   3. Run: cargo check"
echo "   4. If still failing, create a fresh project"
echo ""
echo "📋 To create a fresh project if needed:"
echo "   cargo init --name rust_c2_framework --bin /tmp/fresh_project"
echo "   cp -r src/* /tmp/fresh_project/src/"
echo "   cd /tmp/fresh_project"
echo "   cargo build"