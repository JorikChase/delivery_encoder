#!/bin/bash

# Determine the target executable based on OS
case "$(uname -s)" in
    Linux*|Darwin*)
        # Linux or macOS
        EXECUTABLE="target/release/delivery_encoder"
        ;;
    CYGWIN*|MINGW*|MSYS*)
        # Windows environments
        EXECUTABLE="target/release/delivery_encoder.exe"
        ;;
    *)
        echo "Unsupported operating system"
        exit 1
        ;;
esac

# Check if executable exists
if [ ! -f "$EXECUTABLE" ]; then
    echo "Error: Executable not found at $EXECUTABLE"
    echo "Please build the project first with: cargo build --release"
    exit 1
fi

# Run the executable with all passed arguments
"$EXECUTABLE" "$@"