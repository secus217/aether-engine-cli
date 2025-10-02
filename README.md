# Aether Engine CLI

🚀 A powerful CLI tool for deploying and managing applications on Kubernetes with Aether Engine.

## Installation

Install globally via npm:

```bash
npm install -g aether-engine-cli
```

The CLI will automatically download the appropriate binary for your platform during installation.

## Usage

```bash
# Deploy an application
aether deploy

# Check application status
aether status <app-name>

# View application logs
aether logs <app-name>

# List deployed applications
aether list

# Delete an application
aether delete <app-name>

# Configure CLI
aether config
```

## Supported Platforms

- Linux x64/ARM64
- macOS x64/ARM64 (Intel/Apple Silicon)
- Windows x64

## Features

- 🎯 **Smart Package Manager Detection**: Automatically detects and uses Bun, pnpm, Yarn, or npm
- 🔧 **Flexible Entry Points**: Handles projects with or without start scripts
- 🌐 **Multi-Runtime Support**: Works with Node.js and Bun applications
- 📦 **Pre-built Binaries**: Fast installation with no compilation required
- 🔒 **Secure**: Direct binary downloads from GitHub releases

## Development

This CLI is built with Rust for performance and reliability. The npm package includes pre-built binaries for all supported platforms.

### Building from Source

```bash
git clone https://github.com/secus217/aether-engine-cli.git
cd aether-cli
cargo build --release
```

### Building for All Platforms

```bash
npm run build-all
```

## License

MIT © secus217

## Links

- [GitHub Repository](https://github.com/secus217/aether-engine-cli)
- [npm Package](https://www.npmjs.com/package/aether-engine-cli)
- [Aether Engine](https://github.com/secus217/aether-engine-pub)
# aether-engine-cli
