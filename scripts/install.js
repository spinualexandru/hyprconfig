#!/usr/bin/env node

import { copyFileSync, mkdirSync, writeFileSync, chmodSync, existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { execSync } from 'node:child_process';

const homeDir = homedir();
const projectRoot = process.cwd();

// Paths
const binarySource = join(projectRoot, 'src-tauri/target/release/tauri-app');
const binaryDest = join(homeDir, '.local/bin/hyprconfig');
const iconSource = join(projectRoot, 'src-tauri/icons/128x128.png');
const iconDest = join(homeDir, '.local/share/icons/hicolor/128x128/apps/hyprconfig.png');
const desktopFileDest = join(homeDir, '.local/share/applications/hyprconfig.desktop');

console.log('🚀 Installing Hyprconfig...\n');

// Check if binary exists
if (!existsSync(binarySource)) {
  console.error('❌ Binary not found. Build may have failed.');
  process.exit(1);
}

// Create directories
try {
  mkdirSync(join(homeDir, '.local/bin'), { recursive: true });
  mkdirSync(join(homeDir, '.local/share/applications'), { recursive: true });
  mkdirSync(join(homeDir, '.local/share/icons/hicolor/128x128/apps'), { recursive: true });
  console.log('✓ Created necessary directories');
} catch (error) {
  console.error('❌ Failed to create directories:', error.message);
  process.exit(1);
}

// Copy binary
try {
  copyFileSync(binarySource, binaryDest);
  chmodSync(binaryDest, 0o755);
  console.log('✓ Installed binary to ~/.local/bin/hyprconfig');
} catch (error) {
  if (error.code === 'ETXTBSY') {
    console.error('❌ Failed to install binary: hyprconfig is currently running');
    console.error('   Please close the application and try again.');
  } else {
    console.error('❌ Failed to install binary:', error.message);
  }
  process.exit(1);
}

// Copy icon
try {
  if (existsSync(iconSource)) {
    copyFileSync(iconSource, iconDest);
    console.log('✓ Installed icon');
  } else {
    console.warn('⚠ Icon not found, skipping');
  }
} catch (error) {
  console.warn('⚠ Failed to install icon:', error.message);
}

// Create desktop file
const desktopFileContent = `[Desktop Entry]
Name=Hyprconfig
Comment=GUI configuration tool for Hyprland
Exec=${binaryDest}
Icon=hyprconfig
Type=Application
Categories=Settings;System;
Terminal=false
`;

try {
  writeFileSync(desktopFileDest, desktopFileContent);
  chmodSync(desktopFileDest, 0o644);
  console.log('✓ Created desktop entry');
} catch (error) {
  console.warn('⚠ Failed to create desktop entry:', error.message);
}

// Update desktop database
try {
  execSync('update-desktop-database ~/.local/share/applications 2>/dev/null', { stdio: 'ignore' });
  console.log('✓ Updated desktop database');
} catch (error) {
  // Ignore if command doesn't exist
}

console.log('\n✅ Installation complete!');
console.log('\nYou can now run: hyprconfig');
console.log('Or launch it from your application menu.');
console.log('\nNote: Make sure ~/.local/bin is in your PATH');
