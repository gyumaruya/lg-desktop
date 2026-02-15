<p align="center">
  <img src="assets/banner.svg" alt="lg-desktop" width="800" />
</p>

<p align="center">
  <b>GUI Automation Skill for AI Coding Agents</b><br/>
  <a href="README.ja.md">日本語</a>
</p>

---

GUI automation for AI coding agents -- **97% fewer tokens** than screenshot-based approaches.

Instead of sending full screenshots (765+ tokens each), lg-desktop reads the screen via OCR and returns structured JSON (50-200 tokens). Combined with SHA256-based change detection, a typical 10-step workflow consumes **~285 tokens** versus **7,650+** with conventional screenshot methods.

```
Traditional:  screenshot -> screenshot -> screenshot  (765+ tokens each)
lg-desktop: inspect(text) -> act -> verify(diff)   (50-200 tokens each)
```

## What This Is

lg-desktop is a single skill (`/lg-desktop`) that gives AI coding agents the ability to see and interact with a Docker-based virtual desktop. Install via [skills.sh](https://skills.sh). Works with Claude Code, Cursor, Copilot, and 35+ agents.

```bash
npx skills add gyumaruya/lg-desktop
```

That's it. On first `/lg-desktop up`, the Docker image is automatically built and started.

## Why Token Efficiency Matters

GUI automation with AI agents is notoriously token-expensive. Every screenshot costs 765+ tokens, and a typical automation task needs 10-20 interactions. That adds up fast.

lg-desktop solves this with a **text-first, diff-based** approach:

| Approach | Per Action | 10-Step Task | Savings |
|----------|-----------|--------------|---------|
| Screenshot every time | 765+ tokens | 7,650+ tokens | -- |
| **lg-desktop (text-first)** | **50-200 tokens** | **~285 tokens** | **~97%** |

**How it works:**

1. **Text-First**: OCR reads GUI state as structured JSON with element coordinates -- no image tokens needed
2. **Change Detection**: SHA256 hashing tracks window content; only re-OCRs what changed
3. **Lazy Escalation**: Screenshots are the last resort, not the default

```
Tier 1: inspect (text/JSON)     50-200 tokens  <- DEFAULT
Tier 2: screenshot --crop       ~200 tokens    <- when OCR is ambiguous
Tier 3: screenshot (full)       765+ tokens    <- last resort
```

## Features

- **Text-First Automation**: OCR-powered GUI state inspection returns structured JSON with clickable element coordinates
- **97% Token Reduction**: 3-tier escalation system (text -> crop -> full screenshot) versus screenshot-based approaches
- **Self-Bootstrapping**: First `/lg-desktop up` automatically builds and starts everything
- **Visual Desktop**: Docker container runs Ubuntu desktop (i3 WM) with noVNC web interface
- **Grid Reference System**: 10x10 grid overlay (A1-J10) for easy coordinate specification
- **Change Detection**: SHA256-based state tracking only runs OCR on changed windows

## Architecture

```
AI Coding Agent
  |
  |  /lg-desktop <action>
  |
  v
lg-desktop Skill (SKILL.md)
  |
  |  docker exec
  |
  v
lg-desktop Container (Ubuntu 24.04)
  +-- Xvfb (Virtual display)
  +-- i3 WM (Window manager)
  +-- noVNC (Web UI at localhost:3000)
  +-- lg-inspect (OCR + JSON state)
  +-- lg-grid (Grid overlay)
  +-- xdotool / wmctrl (Automation tools)
  +-- tesseract (OCR engine)
```

## Installation

### Prerequisites

- Docker (with Docker Compose)
- Any AI coding agent supported by [skills.sh](https://skills.sh)

### Install

```bash
npx skills add gyumaruya/lg-desktop
```

### Uninstall

```bash
npx skills remove --skill lg-desktop
docker rm -f lg-desktop 2>/dev/null
docker rmi lg-desktop 2>/dev/null
```

## Quick Start

```
# Start the desktop (auto-bootstraps on first run)
/lg-desktop up

# Inspect desktop state (OCR)
/lg-desktop inspect

# Take a screenshot
/lg-desktop screenshot

# Click at grid position E5
/lg-desktop click E5

# Type text
/lg-desktop type "Hello World"

# Press keyboard shortcut
/lg-desktop key ctrl+s

# Find text and click it
/lg-desktop find-and-click "OK"

# Stop the desktop
/lg-desktop down
```

## Skill Reference

### /lg-desktop

Single skill with subcommands:

**Lifecycle:**
```
/lg-desktop up            # Start (auto-bootstraps on first run)
/lg-desktop down          # Stop
/lg-desktop status        # Check status
/lg-desktop update        # Pull latest + rebuild
```

**Inspect:**
```
/lg-desktop inspect                # Full OCR inspection
/lg-desktop inspect --changes-only # Only changed windows
```

Output example:
```json
{
  "timestamp": "2025-02-12T10:30:00Z",
  "desktop_size": [1280, 1024],
  "focused_window": "0x1234567",
  "windows": [
    {
      "id": "0x1234567",
      "title": "Terminal",
      "geometry": {"x": 0, "y": 0, "w": 640, "h": 512},
      "ocr_text": "user@host:~$ _",
      "elements": [
        {
          "text": "user@host",
          "x": 10, "y": 20, "w": 80, "h": 15,
          "confidence": 95.0
        }
      ],
      "changed": true
    }
  ],
  "changes_since_last": ["0x1234567"]
}
```

**Click:**
```
/lg-desktop click A5        # Grid reference
/lg-desktop click 500,300   # Pixel coordinates
```

**Type:**
```
/lg-desktop type "Hello World"
```

**Keyboard:**
```
/lg-desktop key Return
/lg-desktop key ctrl+s
/lg-desktop key alt+F4
```

**Screenshot:**
```
/lg-desktop screenshot                # Basic
/lg-desktop screenshot --grid         # With grid overlay
/lg-desktop screenshot --crop 100,200,300,400  # Crop region
```

**Find and Click:**
```
/lg-desktop find-and-click "OK button"
```

**Wait:**
```
/lg-desktop wait-for --window "Firefox"
/lg-desktop wait-for --text "Loading complete"
/lg-desktop wait-for --gone "Please wait..."
```

**Assert:**
```
/lg-desktop assert --window "Settings"
/lg-desktop assert --no-window "Error"
/lg-desktop assert --text "Success"
```

**Run Command:**
```
/lg-desktop run xterm
```

**Copy Files:**
```
/lg-desktop copy-from /tmp/output.txt ./output.txt
/lg-desktop copy-to ./input.txt /tmp/input.txt
```

## Technical Details

### Container Specifications

| Component | Detail |
|-----------|--------|
| Base Image | Ubuntu 24.04 |
| Display Server | Xvfb (virtual framebuffer) |
| Window Manager | i3 (tiling, OCR-optimized) |
| VNC Server | x11vnc + noVNC (web interface) |
| Resolution | 1280x1024 (configurable) |
| OCR Engine | Tesseract (eng + jpn) |

### Binaries

Two Rust binaries compiled as static Linux musl executables:

- **lg-inspect**: OCR-based desktop state inspection with SHA256 change detection
- **lg-grid**: 10x10 grid overlay generator for screenshots

### Shared Directory

The container mounts `/tmp/lg-desktop-share` (host) to `/shared` (container) for:
- State persistence (`lg-state.json`)
- Screenshot storage
- File exchange between host and container

## Advanced Usage

### Custom Recipes

Create automation recipes in `docker/recipes/`:

```bash
#!/bin/bash
# recipe: open-firefox
xdotool key alt+Return
sleep 0.3
xdotool type --clearmodifiers "firefox"
xdotool key Return
sleep 2
lg-inspect --changes-only
```

Execute:
```
/lg-desktop run /usr/local/lib/lg-recipes/open-firefox.sh
```

### Java SWT Applications

For Java SWT apps (Eclipse, proprietary tools), use `--window` flag with xdotool:

```bash
WID=$(wmctrl -l | grep "Eclipse" | awk '{print $1}')
xdotool mousemove --sync --window $WID 500 300 click 1
```

### Change Detection

lg-inspect uses SHA256 hashing of window screenshots to detect changes:
- First run: OCR all windows
- Subsequent runs: Only OCR windows with changed content
- `--changes-only` flag returns only changed windows in JSON

## Troubleshooting

### Desktop not starting

```bash
docker info                    # Check Docker
docker logs lg-desktop         # Check container logs

# Rebuild (via skill)
/lg-desktop down
/lg-desktop up
```

### OCR not working

```bash
docker exec lg-desktop tesseract --version
docker exec -e DISPLAY=:1 lg-desktop scrot /tmp/test.png
docker exec lg-desktop tesseract /tmp/test.png stdout
```

### Click not working

- Focus the window first: `xdotool windowfocus --sync <WID>`
- For Java SWT apps, use `--window` flag
- Increase sleep time between actions
- Verify coordinates with `screenshot --grid`

## Performance

Typical token consumption per action:

| Method | Tokens |
|--------|--------|
| inspect (full) | 100-200 |
| inspect --changes-only | 30-80 |
| wmctrl -l (verify) | 15-30 |
| screenshot (full) | 765+ |
| screenshot --crop | 200-400 |

10-step automation example:
- 1x full inspect + 9x changes-only verify: ~285 tokens
- Old approach (10x screenshot): 7650+ tokens
- **~96% reduction**

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test with `cargo test` and `cargo clippy`
5. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Acknowledgments

- Extracted from [RushValley](https://github.com/gyumaruya/RushValley) - AI agent embodiment project
- Built with Rust, Docker, Tesseract, and noVNC

## Links

- **Repository**: https://github.com/gyumaruya/lg-desktop
- **Issues**: https://github.com/gyumaruya/lg-desktop/issues
- **skills.sh**: https://skills.sh
