# 12

A Loquat plugin

## Installation

### For Rust Plugins
```bash
cargo build --release
# Copy the compiled library to your plugins directory
cp target/release/lib12.dll ./plugins/  # Windows
cp target/release/lib12.so ./plugins/     # Linux
cp target/release/lib12.dylib ./plugins/ # macOS
```

### For Python Plugins
```bash
pip install -r requirements.txt
# Copy the plugin file to your plugins directory
cp main.py ./plugins/
```

### For JavaScript/TypeScript Plugins
```bash
npm install
npm run build
# Copy the compiled file to your plugins directory
cp dist/index.js ./plugins/
```

## Configuration

Add the following to your `config.toml`:

```toml
[plugins.12]
enabled = true
auto_load = true
```

## Usage

The plugin will be automatically loaded by the Loquat framework.

## Author

Your Name
