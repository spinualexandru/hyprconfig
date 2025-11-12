export default function About() {
  return (
    <div className="p-8">
      <h1 className="text-3xl font-semibold mb-6">About</h1>

      <div className="space-y-6">
        <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6 shadow-sm">
          <h2 className="text-xl font-semibold mb-3">Hyprconfig</h2>
          <p className="text-gray-600 dark:text-gray-400 mb-2">Version 0.1.0</p>
          <p className="text-gray-600 dark:text-gray-400">
            A GUI application for configuring Hyprland compositor built with Tauri 2.0.
          </p>
        </div>

        <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6 shadow-sm">
          <h3 className="text-lg font-semibold mb-3">Technology Stack</h3>
          <ul className="space-y-2 text-gray-600 dark:text-gray-400">
            <li>• Frontend: React 18 + TypeScript</li>
            <li>• Backend: Rust + Tauri 2.0</li>
            <li>• UI: Tailwind CSS + Lucide Icons</li>
            <li>• Hyprland Integration: hyprland crate</li>
          </ul>
        </div>

        <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6 shadow-sm">
          <h3 className="text-lg font-semibold mb-3">License</h3>
          <p className="text-gray-600 dark:text-gray-400">MIT License</p>
        </div>
      </div>
    </div>
  );
}
