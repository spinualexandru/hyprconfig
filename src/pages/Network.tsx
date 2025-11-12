export default function Network() {
  return (
    <div className="p-6">
      <h2 className="text-2xl font-semibold mb-6">Network</h2>
      <div className="space-y-6">
        <div className="space-y-2">
          <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100">Wi-Fi</h3>
          <div className="space-y-3">
            <div className="flex items-center justify-between p-4 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800">
              <div>
                <p className="font-medium text-gray-900 dark:text-gray-100">Home Network</p>
                <p className="text-sm text-gray-500 dark:text-gray-400">Connected</p>
              </div>
              <div className="h-2 w-2 rounded-full bg-green-500"></div>
            </div>
          </div>
        </div>
        <div className="space-y-2">
          <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100">Ethernet</h3>
          <div className="p-4 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800">
            <p className="text-sm text-gray-500 dark:text-gray-400">Not connected</p>
          </div>
        </div>
      </div>
    </div>
  );
}
