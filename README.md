# glfos-samba-shares
GUI to manage samba shares on glfos

## Features

- **Add Samba Shares**: Create new Samba shares with a user-friendly form
- **Edit Shares**: Modify existing share configurations
- **List Shares**: View all configured Samba shares
- **NixOS Integration**: Automatically updates `/etc/nixos/customConfig/default.nix`
- **Nix Parser**: Uses `rnix` library to parse nix config file
- **User/Group Selection**: Choose from system users and groups
- **Path Browser**: Native folder picker for share paths
- **Validation**: Form validation for required fields

## Build

### With Nix

```bash
# Compiler le paquet
nix build

# Exécuter directement
nix run

# Entrer dans l'environnement de développement
nix develop
```

### With Cargo

```bash
# Entrer d'abord dans l'environnement de développement
nix develop

# Compiler
cargo build --release

# Or manually with sudo, preserving environment
sudo -E ./target/release/samba-share
```

**Note:** The application needs root privileges to modify `/etc/nixos/customConfig/default.nix`. When running with `sudo`, you must preserve the `XDG_DATA_DIRS` environment variable to avoid GSettings schema errors. See [DEVELOPMENT.md](DEVELOPMENT.md) for more details.

## Testing

The project includes a comprehensive test suite to prevent regressions.

### Run all tests

```bash
./run-tests.sh
```

Or with cargo:

```bash
cargo test
```

### Run specific test

```bash
./run-tests.sh --test test_add_first_share_to_existing_settings
```

### Run with verbose output

```bash
./run-tests.sh --verbose
```

 

## Development

1. Make changes to the code
2. Run tests to ensure no regressions:
   ```bash
   ./run-tests.sh
   ```
3. Build and test the application:
   ```bash
   cargo build
   cargo run
   ```

## License

GPL-3.0-or-later