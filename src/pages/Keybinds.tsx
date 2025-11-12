export default function Keybinds() {
  return (
    <div className="p-6">
      <h2 className="text-2xl font-semibold mb-6">Keybinds</h2>
      <div className="space-y-4">
        <div className="flex items-center justify-between p-3 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800">
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">Open Terminal</span>
          <kbd className="px-2 py-1 text-xs font-semibold text-gray-800 bg-gray-100 border border-gray-200 rounded-md dark:bg-gray-600 dark:text-gray-100 dark:border-gray-500">
            Super + Return
          </kbd>
        </div>
        <div className="flex items-center justify-between p-3 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800">
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">Close Window</span>
          <kbd className="px-2 py-1 text-xs font-semibold text-gray-800 bg-gray-100 border border-gray-200 rounded-md dark:bg-gray-600 dark:text-gray-100 dark:border-gray-500">
            Super + Q
          </kbd>
        </div>
        <div className="flex items-center justify-between p-3 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800">
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">Toggle Fullscreen</span>
          <kbd className="px-2 py-1 text-xs font-semibold text-gray-800 bg-gray-100 border border-gray-200 rounded-md dark:bg-gray-600 dark:text-gray-100 dark:border-gray-500">
            Super + F
          </kbd>
        </div>
        <div className="flex items-center justify-between p-3 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800">
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">Move Focus</span>
          <kbd className="px-2 py-1 text-xs font-semibold text-gray-800 bg-gray-100 border border-gray-200 rounded-md dark:bg-gray-600 dark:text-gray-100 dark:border-gray-500">
            Super + Arrow Keys
          </kbd>
        </div>
      </div>
    </div>
  );
}
