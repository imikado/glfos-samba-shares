# Testing Guide

## Quick Start

Run all tests:
```bash
./run-tests.sh
```

## Test Suite Overview

The project has **10 comprehensive tests** to ensure no regressions:

### Unit Tests (2 tests)

Located in `src/samba/share_config.rs`:

1. ✅ `test_write_adds_share_correctly`
   - Adds "myTEst2" to existing config with "myTest1a"
   - Validates proper brace counting
   - Ensures both shares present

2. ✅ `test_write_with_no_existing_settings`
   - Creates entire `services.samba.settings` section
   - Tests configuration from scratch

### Integration Tests (8 tests)

Located in `tests/integration_tests.rs`:

1. ✅ `test_add_first_share_to_existing_settings`
   - Adds second share to config with one share
   - Validates insertion position

2. ✅ `test_add_third_share_maintains_order`
   - Adds third share
   - Verifies order: share1 → share2 → share3

3. ✅ `test_create_samba_section_when_missing`
   - Creates full section in minimal config
   - Validates structure creation

4. ✅ `test_update_share_removes_old_and_adds_new`
   - Updates existing share
   - Ensures old removed, new added
   - Other shares unaffected

5. ✅ `test_brace_counting_with_nested_structures`
   - Tests nested brace counting
   - Validates parser accuracy

6. ✅ `test_empty_settings_section`
   - Handles empty `settings = {}`
   - Edge case testing

7. ✅ `test_special_characters_in_paths`
   - Paths with spaces work
   - Example: "/home/user/My Documents/Shared Folder"

8. ✅ `test_configuration_values`
   - Validates `yes`/`no` format (not `true`/`false`)
   - NixOS compliance

## Example Test Case

The main test validates this transformation:

**Input:**
```nix
services.samba = {
  settings = {
    "myTest1a" = {
      path = "/home/mika/testshare";
      browseable = yes;
      "read only" = yes;
      "guest ok" = no;
      "force user" = "_apt";
      "force group" = "adm";
    };
  };
};
```

**After adding "myTEst2":**
```nix
services.samba = {
  settings = {
    "myTest1a" = {
      path = "/home/mika/testshare";
      browseable = yes;
      "read only" = yes;
      "guest ok" = no;
      "force user" = "_apt";
      "force group" = "adm";
    };
    "myTEst2" = {
      path = "/home/mika/testShare2";
      browseable = yes;
      "read only" = yes;
      "guest ok" = no;
      "force user" = "_apt";
      "force group" = "adm";
    };
  };
};
```

## Running Specific Tests

Test a specific scenario:
```bash
./run-tests.sh --test test_add_third_share_maintains_order
```

With verbose output:
```bash
./run-tests.sh --verbose
```

## Expected Output

```
========================================
  Samba Share Manager - Test Suite
========================================

Running: cargo test -- --nocapture

     Running unittests src/main.rs
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running tests/integration_tests.rs
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

========================================
  ✅ All tests passed!
========================================
```

## What These Tests Protect Against

### Regression Prevention

1. **Brace Counting Errors**
   - ❌ Before fix: `settings_brace_count = 0` (missed opening brace)
   - ✅ After fix: `settings_brace_count = trimmed.matches('{').count()`

2. **Wrong Insertion Position**
   - Tests ensure shares inserted before `};` closing
   - Prevents malformed configuration

3. **Lost Shares**
   - Validates all existing shares preserved
   - New shares added correctly

4. **Structure Corruption**
   - Ensures proper nesting maintained
   - NixOS syntax compliance

### Continuous Integration

Run tests:
- ✅ Before every commit
- ✅ In CI/CD pipelines
- ✅ Before releases
- ✅ After refactoring

## Adding New Tests

When adding features:

1. Write test first (TDD)
2. Implement feature
3. Ensure test passes
4. Update this documentation

### Test Template

```rust
#[test]
fn test_new_feature() {
    // Arrange: Setup test data
    let config = "...";

    // Act: Perform operation
    let result = perform_operation(config);

    // Assert: Verify results
    assert!(result.is_ok(), "Should succeed");
    assert_eq!(result.value, expected, "Should match expected");
}
```

## Debugging Failed Tests

If tests fail:

1. Read the assertion message
2. Check the test output with `--nocapture`
3. Review recent code changes
4. Verify brace counting logic
5. Check NixOS syntax compliance

Example:
```bash
cargo test test_add_first_share -- --nocapture
```

## Performance

Tests run in **< 1 second**:
- Fast feedback loop
- No external dependencies
- Pure logic testing

## Coverage

Current coverage:
- ✅ Share addition
- ✅ Share updates
- ✅ Share deletion (via update)
- ✅ Structure creation
- ✅ Brace counting
- ✅ Edge cases

Future additions:
- [ ] Delete share functionality
- [ ] Duplicate share detection
- [ ] Invalid path handling
- [ ] Permission validation
