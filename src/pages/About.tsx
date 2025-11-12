export default function About() {
  return (
    <div className="p-6">
      <h2 className="text-2xl font-semibold mb-6">About</h2>
      <div className="space-y-6">
        <div className="flex items-center space-x-4">
          <div className="w-20 h-20 bg-gradient-to-br from-blue-500 to-purple-600 rounded-2xl flex items-center justify-center">
            <span className="text-3xl font-bold text-white">H</span>
          </div>
          <div>
            <h3 className="text-xl font-semibold text-gray-900 dark:text-gray-100">Hyprland Configurator</h3>
            <p className="text-sm text-gray-500 dark:text-gray-400">Version 0.1.0</p>
          </div>
        </div>
        <div className="space-y-2">
          <p className="text-sm text-gray-700 dark:text-gray-300">
            A modern, user-friendly configuration tool for Hyprland window manager.
          </p>
          <p className="text-sm text-gray-700 dark:text-gray-300">
            Built with Tauri, React, and Radix UI.
          </p>
        </div>
        <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
          <p className="text-xs text-gray-500 dark:text-gray-400">
            © 2025 Hyprland Configurator. All rights reserved.
          </p>
        </div>
      </div>
    </div>
  );
}
