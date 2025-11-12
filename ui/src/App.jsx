import React from 'react';
import * as Tabs from '@radix-ui/react-tabs';
import * as ScrollArea from '@radix-ui/react-scroll-area';
import DisplayPage from './components/DisplayPage';
import './App.css';

function App() {
  return (
    <div className="app">
      <Tabs.Root defaultValue="display" orientation="vertical" className="tabs-root">
        <Tabs.List className="tabs-list" aria-label="Settings sections">
          <div className="sidebar-header">
            <h2>HyprConfig</h2>
          </div>
          <Tabs.Trigger value="appearance" className="tabs-trigger">
            <span className="icon">🎨</span>
            <span>Appearance</span>
          </Tabs.Trigger>
          <Tabs.Trigger value="keyboard" className="tabs-trigger">
            <span className="icon">⌨️</span>
            <span>Keyboard</span>
          </Tabs.Trigger>
          <Tabs.Trigger value="display" className="tabs-trigger">
            <span className="icon">🖥️</span>
            <span>Display</span>
          </Tabs.Trigger>
          <Tabs.Trigger value="network" className="tabs-trigger">
            <span className="icon">🌐</span>
            <span>Network</span>
          </Tabs.Trigger>
          <Tabs.Trigger value="plugins" className="tabs-trigger">
            <span className="icon">🔌</span>
            <span>Plugins</span>
          </Tabs.Trigger>
          <Tabs.Trigger value="about" className="tabs-trigger">
            <span className="icon">ℹ️</span>
            <span>About</span>
          </Tabs.Trigger>
        </Tabs.List>

        <ScrollArea.Root className="scroll-area-root">
          <ScrollArea.Viewport className="scroll-area-viewport">
            <Tabs.Content value="appearance" className="tabs-content">
              <h1>Appearance</h1>
              <p>Configure the appearance settings for Hyprland.</p>
            </Tabs.Content>

            <Tabs.Content value="keyboard" className="tabs-content">
              <h1>Keyboard</h1>
              <p>Configure keyboard settings and keybinds for Hyprland.</p>
            </Tabs.Content>

            <Tabs.Content value="display" className="tabs-content">
              <DisplayPage />
            </Tabs.Content>

            <Tabs.Content value="network" className="tabs-content">
              <h1>Network</h1>
              <p>Configure network settings.</p>
            </Tabs.Content>

            <Tabs.Content value="plugins" className="tabs-content">
              <h1>Plugins</h1>
              <p>Manage Hyprland plugins.</p>
            </Tabs.Content>

            <Tabs.Content value="about" className="tabs-content">
              <h1>About</h1>
              <div className="about-content">
                <h2>HyprConfig</h2>
                <p>Version 0.1.0</p>
                <p>A modern GUI configuration tool for Hyprland.</p>
                <p>Built with Tauri, React, and Radix UI.</p>
              </div>
            </Tabs.Content>
          </ScrollArea.Viewport>
          <ScrollArea.Scrollbar className="scroll-area-scrollbar" orientation="vertical">
            <ScrollArea.Thumb className="scroll-area-thumb" />
          </ScrollArea.Scrollbar>
        </ScrollArea.Root>
      </Tabs.Root>
    </div>
  );
}

export default App;
