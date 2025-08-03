# Default target directory for binaries
TARGET_DIR = $(HOME)/bin

# Binary names
BINARIES = audiotools fmtr

.PHONY: all build install clean

# Default target
all: install

# Build all binaries in release mode
build:
	cargo build --release

# Install all binaries to target directory
install: build
	@mkdir -p $(TARGET_DIR)
	@for binary in $(BINARIES); do \
		cp target/release/$$binary $(TARGET_DIR)/ && \
		echo "Installed $$binary to $(TARGET_DIR)" ; \
	done

# Clean build artifacts
clean:
	cargo clean
