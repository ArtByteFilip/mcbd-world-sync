# Minecraft Bedrock World Sync

A program for synchronizing Minecraft Bedrock Edition worlds between devices.

## Features

- Automatic detection of Minecraft Bedrock worlds
- Real-time monitoring of world changes
- Synchronization of changes between devices
- Automatic conflict resolution
- Support for multiple devices
- Configurable via JSON file

## Requirements

- Windows 10/11
- Minecraft Bedrock Edition
- Rust (for compilation)
- Administrator privileges (for accessing Minecraft files)

## Installation

1. Download or clone the repository
2. Install Rust from [rustup.rs](https://rustup.rs)
3. Compile the program:
   ```bash
   cargo build --release
   ```

## Configuration

Modify the `config.json` file according to your needs:

```json
{
    "server": {
        "port": 8080,
        "host": "0.0.0.0"
    },
    "sync": {
        "devices": [
            {
                "name": "local",
                "address": "127.0.0.1:8080"
            }
        ],
        "conflict_resolution": "newest",
        "sync_interval": 60
    },
    "paths": {
        "minecraft_worlds": "C:\\Users\\USERNAME\\AppData\\Local\\Packages\\Microsoft.MinecraftUWP_8wekyb3d8bbwe\\LocalState\\games\\com.mojang\\minecraftWorlds"
    }
}
```

To synchronize between devices, add additional devices to the `devices` section:

```json
{
    "name": "remote",
    "address": "IP_ADDRESS:8080"
}
```

## Usage

1. Run the program with administrator privileges:
   - Right-click on `target/release/mcbd-world-sync.exe`
   - Select "Run as administrator"

2. The program automatically:
   - Finds Minecraft worlds
   - Starts monitoring for changes
   - Synchronizes changes with other devices

## Troubleshooting

### Access Denied
If you see an "Access is denied" error:
1. Make sure the program is running with administrator privileges
2. Check if Minecraft Bedrock Edition is installed
3. Verify that the path in `config.json` is correct

### Synchronization Not Working
1. Check if both computers are on the same network
2. Verify that port 8080 is not blocked by the firewall
3. Check the IP addresses in the configuration

## Security

- The program requires administrator privileges to access Minecraft files
- Synchronization only occurs within the local network
- All files are synchronized in their original form

## License

MIT License 