#!/usr/bin/env bash
# Test runner script for Samba Share Manager

set -e

echo "========================================"
echo "  Samba Share Manager - Test Suite"
echo "========================================"
echo ""

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found. Please install Rust."
    exit 1
fi

# Parse arguments
VERBOSE=false
TEST_NAME=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -t|--test)
            TEST_NAME="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -v, --verbose    Show verbose output"
            echo "  -t, --test NAME  Run specific test by name"
            echo "  -h, --help       Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                              # Run all tests"
            echo "  $0 --verbose                    # Run with verbose output"
            echo "  $0 --test test_write_adds_share # Run specific test"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Build the test command
CMD="cargo test"

if [ -n "$TEST_NAME" ]; then
    CMD="$CMD $TEST_NAME"
fi

if [ "$VERBOSE" = true ]; then
    CMD="$CMD --verbose"
fi

CMD="$CMD -- --nocapture"

# Run tests
echo "Running: $CMD"
echo ""

$CMD

EXIT_CODE=$?

echo ""
if [ $EXIT_CODE -eq 0 ]; then
    echo "========================================"
    echo "  ✅ All tests passed!"
    echo "========================================"
else
    echo "========================================"
    echo "  ❌ Tests failed!"
    echo "========================================"
fi

exit $EXIT_CODE
