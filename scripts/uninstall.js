#!/usr/bin/env node

import { unlinkSync, existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { execSync } from 'node:child_process';

const homeDir = homedir();

// Paths
const binaryPath = join(homeDir, '.local/bin/hyprconfig');
const iconPath = join(homeDir, '.local/share/icons/hicolor/128x128/apps/hyprconfig.png');
const desktopFilePath = join(homeDir, '.local/share/applications/hyprconfig.desktop');

console.log('🗑️  Uninstalling Hyprconfig...\n');

let filesRemoved = 0;

// Remove binary
try {
  if (existsSync(binaryPath)) {
    unlinkSync(binaryPath);
    console.log('✓ Removed binary from ~/.local/bin/hyprconfig');
    filesRemoved++;
  } else {
    console.log('⚠ Binary not found (already removed?)');
  }
} catch (error) {
  console.error('❌ Failed to remove binary:', error.message);
}

// Remove icon
try {
  if (existsSync(iconPath)) {
    unlinkSync(iconPath);
    console.log('✓ Removed icon');
    filesRemoved++;
  } else {
    console.log('⚠ Icon not found (already removed?)');
  }
} catch (error) {
  console.error('❌ Failed to remove icon:', error.message);
}

// Remove desktop file
try {
  if (existsSync(desktopFilePath)) {
    unlinkSync(desktopFilePath);
    console.log('✓ Removed desktop entry');
    filesRemoved++;
  } else {
    console.log('⚠ Desktop entry not found (already removed?)');
  }
} catch (error) {
  console.error('❌ Failed to remove desktop entry:', error.message);
}

// Update desktop database
try {
  execSync('update-desktop-database ~/.local/share/applications 2>/dev/null', { stdio: 'ignore' });
  console.log('✓ Updated desktop database');
} catch {
  // Ignore if command doesn't exist
}

if (filesRemoved > 0) {
  console.log('\n✅ Uninstallation complete!');
} else {
  console.log('\n⚠️  No files were found to remove. Hyprconfig may not be installed.');
}
