# Test Suite for Samba Share Manager

This directory contains integration tests to prevent regressions and ensure the core functionality works correctly.

## Running Tests

Run all tests:
```bash
cargo test
```

Run tests with output:
```bash
cargo test -- --nocapture
```

Run specific test:
```bash
cargo test test_add_first_share_to_existing_settings
```

Run with verbose output:
```bash
cargo test --verbose
```

## Test Coverage

### Unit Tests (in `src/samba/share_config.rs`)

1. **test_write_adds_share_correctly**
   - Verifies adding a new share to an existing `services.samba.settings` section
   - Ensures proper brace counting and insertion position
   - Validates that both old and new shares are present

2. **test_write_with_no_existing_settings**
   - Tests creating the entire `services.samba.settings` section when it doesn't exist
   - Validates proper structure creation

### Integration Tests (in `tests/integration_tests.rs`)

1. **test_add_first_share_to_existing_settings**
   - Adds a second share to a configuration with one existing share
   - Verifies the share is inserted at the correct position
   - Ensures both shares are present in the final configuration

2. **test_add_third_share_maintains_order**
   - Adds a third share to a configuration with two existing shares
   - Validates that share order is preserved (share1, share2, share3)
   - Tests that the insertion logic handles multiple existing shares

3. **test_create_samba_section_when_missing**
   - Creates the entire `services.samba.settings` section in a minimal config
   - Validates proper nesting and structure creation
   - Ensures the new share is properly formatted

4. **test_update_share_removes_old_and_adds_new**
   - Simulates updating an existing share
   - Verifies the old share is removed
   - Ensures the updated share is added at the same position
   - Validates that other shares are not affected

5. **test_brace_counting_with_nested_structures**
   - Tests the brace counting logic with nested configuration blocks
   - Ensures the parser correctly identifies the closing brace of the settings section
   - Validates handling of complex nested structures

6. **test_empty_settings_section**
   - Tests adding a share to an empty `services.samba.settings` block
   - Ensures the logic handles edge cases where settings exist but have no shares

7. **test_special_characters_in_paths**
   - Validates that paths with spaces and special characters are handled correctly
   - Ensures proper quoting and formatting

8. **test_configuration_values**
   - Verifies that boolean values use NixOS format (`yes`/`no` instead of `true`/`false`)
   - Validates proper formatting of configuration values

## What These Tests Prevent

These tests help prevent regressions in:

1. **Brace Counting Logic**: Ensures proper tracking of nested braces in NixOS configuration
2. **Share Addition**: Validates that new shares are added at the correct position
3. **Share Updates**: Verifies that updating shares replaces the old configuration
4. **Structure Creation**: Ensures proper creation of `services.samba.settings` when missing
5. **Order Preservation**: Validates that share order is maintained when adding new shares
6. **Edge Cases**: Tests empty settings sections and special characters

## Expected Output

When all tests pass, you should see:
```
running 2 tests
test samba::share_config::tests::test_write_adds_share_correctly ... ok
test samba::share_config::tests::test_write_with_no_existing_settings ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

running 8 tests
test test_add_first_share_to_existing_settings ... ok
test test_add_third_share_maintains_order ... ok
test test_brace_counting_with_nested_structures ... ok
test test_configuration_values ... ok
test test_create_samba_section_when_missing ... ok
test test_empty_settings_section ... ok
test test_special_characters_in_paths ... ok
test test_update_share_removes_old_and_adds_new ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Total: **10 tests** covering the core functionality

## Adding New Tests

When adding new features:

1. Add unit tests in `src/samba/share_config.rs` for module-specific logic
2. Add integration tests in `tests/integration_tests.rs` for end-to-end scenarios
3. Follow the existing test naming convention: `test_<what_is_being_tested>`
4. Include assertions that verify both positive cases and edge cases

## Continuous Integration

These tests should be run:
- Before committing changes
- In CI/CD pipelines
- Before releasing new versions
- After refactoring existing code
