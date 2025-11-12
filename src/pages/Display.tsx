export default function Display() {
  return (
    <div className="p-6">
      <h2 className="text-2xl font-semibold mb-6">Display</h2>
      <div className="space-y-6">
        <div className="space-y-2">
          <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Resolution
          </label>
          <select className="w-full max-w-md px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500">
            <option>1920x1080</option>
            <option>2560x1440</option>
            <option>3840x2160</option>
          </select>
        </div>
        <div className="space-y-2">
          <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Refresh Rate
          </label>
          <select className="w-full max-w-md px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500">
            <option>60 Hz</option>
            <option>75 Hz</option>
            <option>144 Hz</option>
            <option>165 Hz</option>
          </select>
        </div>
        <div className="space-y-2">
          <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Scale
          </label>
          <input
            type="range"
            min="100"
            max="200"
            step="25"
            defaultValue="100"
            className="w-full max-w-md"
          />
          <p className="text-xs text-gray-500 dark:text-gray-400">100%</p>
        </div>
      </div>
    </div>
  );
}
