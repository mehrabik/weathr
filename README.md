# weathr

A terminal weather app with ASCII animations.

## Contents

- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [License](#license)

## Installation

### Build

You need Rust installed.

```bash
git clone https://github.com/veirt/weathr.git
cd weathr
cargo install --path .
```

## Configuration

The config file is at `~/.config/weathr/config.toml`.

### Setup

```bash
mkdir -p ~/.config/weathr
```

Edit `~/.config/weathr/config.toml`:

```toml
[location]
latitude = 40.7128
longitude = -74.0060
```

### Example Locations

```toml
# Tokyo, Japan
latitude = 35.6762
longitude = 139.6503

# Sydney, Australia
latitude = -33.8688
longitude = 151.2093
```

## Usage

Run:

```bash
weathr
```

## License

GPL-3.0-or-later

## Credits

### ASCII Art
- **Source**: https://www.asciiart.eu/
- **House**: Joan G. Stark
- **Sun**: Hayley Jane Wakenshaw (Flump)
- **Moon**: Joan G. Stark

*Note: If any ASCII art is uncredited or misattributed, it belongs to the original owner.*
