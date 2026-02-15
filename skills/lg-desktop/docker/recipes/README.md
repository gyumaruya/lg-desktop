# lg-desktop Recipes

Automation recipes for the Docker virtual desktop environment.

## Recipe Format

Each recipe is a shell script in this directory that automates a common GUI workflow.

### Structure

```
recipes/
  README.md          # This file
  <name>.sh          # Recipe scripts
```

### Authoring Guide

1. Scripts receive the shared directory as `$SHARED_DIR` (default: `/shared`)
2. Use `lg-inspect` for text-first state detection before acting
3. Use `xdotool` for mouse/keyboard actions
4. Use `--window $WID` flag for Java SWT apps
5. Add `sleep` between actions for GUI responsiveness
6. Return JSON result to stdout for programmatic consumption

### Example

```bash
#!/bin/bash
# recipe: open-terminal
# description: Open a new terminal window

xdotool key Mod1+Return
sleep 0.5
lg-inspect --changes-only
```

### Conventions

- Filename = recipe name (e.g., `open-terminal.sh`)
- First comment block documents the recipe
- Exit 0 on success, non-zero on failure
- Output structured JSON when possible

## Usage

Recipes are executed inside the container:

```bash
docker exec -e DISPLAY=:1 lg-desktop /usr/local/lib/lg-recipes/<recipe-name>.sh
```

Or via the Claude Code plugin:

```
/lg-desktop run /usr/local/lib/lg-recipes/<recipe-name>.sh
```
