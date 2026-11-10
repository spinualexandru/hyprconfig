# https://just.systems

bin := env_var('HOME') / ".local/bin/hyprconfig"
icon := env_var('HOME') / ".local/share/icons/hicolor/128x128/apps/hyprconfig.png"
desktop := env_var('HOME') / ".local/share/applications/hyprconfig.desktop"

default: run

# Run an optimized build
run:
    cargo run --release

# Run a debug build
dev:
    cargo run

# Formatting, lints and tests, as they should pass before a commit
check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test

# Render every scene headless into artifacts/ (PNG and layout tree)
snapshots theme="dark":
    cargo build
    mkdir -p artifacts
    for s in $(target/debug/hyprconfig --list-scenes); do \
        target/debug/hyprconfig --scene "$s" --theme {{theme}} --size 1100x760 --at 1 \
            --snap "artifacts/$s-{{theme}}.png" --tree "artifacts/$s-{{theme}}.json" > /dev/null; \
    done

# Build and install for the current user (~/.local)
install:
    cargo build --release
    install -Dm755 target/release/hyprconfig {{bin}}
    install -Dm644 assets/hyprconfig.png {{icon}}
    mkdir -p "$(dirname {{desktop}})"
    printf '[Desktop Entry]\nName=Hyprconfig\nComment=GUI configuration tool for Hyprland\nExec=%s\nIcon=hyprconfig\nType=Application\nCategories=Settings;System;\nTerminal=false\nStartupWMClass=hyprconfig\n' "{{bin}}" > {{desktop}}
    -update-desktop-database "$(dirname {{desktop}})" 2>/dev/null
    @echo "Installed {{bin}}. Make sure ~/.local/bin is in your PATH."

uninstall:
    rm -f {{bin}} {{icon}} {{desktop}}
    -update-desktop-database "$(dirname {{desktop}})" 2>/dev/null
