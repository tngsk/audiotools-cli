# Default target directory for binaries
TARGET_DIR = $(HOME)/bin

# List of all independent CLI binaries
CLI_BINARIES = \
	spectrum-cli \
	convert-cli \
	info-cli \
	loudness-cli \
	normalize-cli \
	waveform-cli

.PHONY: all build install clean

# Default target
all: install

# Build all CLI binaries in release mode
build:
	@echo "Building all CLI binaries..."
cargo build --release

# Install all CLI binaries to target directory
install: build
	@echo "Installing all CLI binaries to $(TARGET_DIR)..."
	@mkdir -p $(TARGET_DIR)
	@for binary in $(CLI_BINARIES); do \
		cp target/release/$$binary $(TARGET_DIR)/ && \
		echo "Installed $$binary to $(TARGET_DIR)" ; \
	done

# Clean build artifacts for the entire workspace
clean:
	@echo "Cleaning build artifacts..."
cargo clean
