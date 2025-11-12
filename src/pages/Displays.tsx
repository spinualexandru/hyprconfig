import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Monitor } from "lucide-react";

interface MonitorInfo {
  id: number;
  name: string;
  description: string;
  width: number;
  height: number;
  refresh_rate: number;
  x: number;
  y: number;
  scale: number;
  transform: string;
  active_workspace_id: number;
  active_workspace_name: string;
}

export default function Displays() {
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadMonitors();
  }, []);

  const loadMonitors = async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<MonitorInfo[]>("get_monitors");
      setMonitors(result);
    } catch (err) {
      setError(err as string);
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return (
      <div className="p-8">
        <h1 className="text-3xl font-semibold mb-6">Displays</h1>
        <p className="text-gray-600 dark:text-gray-400">Loading monitor information...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-8">
        <h1 className="text-3xl font-semibold mb-6">Displays</h1>
        <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
          <p className="text-red-800 dark:text-red-200">Error loading monitors: {error}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="p-8">
      <h1 className="text-3xl font-semibold mb-6">Displays</h1>

      <div className="space-y-4">
        {monitors.length === 0 ? (
          <p className="text-gray-600 dark:text-gray-400">No monitors detected.</p>
        ) : (
          monitors.map((monitor) => (
            <div
              key={monitor.id}
              className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6 shadow-sm"
            >
              <div className="flex items-start gap-4">
                <div className="p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
                  <Monitor className="w-6 h-6 text-blue-600 dark:text-blue-400" />
                </div>

                <div className="flex-1">
                  <h3 className="text-xl font-semibold mb-1">{monitor.name}</h3>
                  <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
                    {monitor.description}
                  </p>

                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <span className="text-gray-500 dark:text-gray-400">Resolution:</span>
                      <span className="ml-2 font-medium">
                        {monitor.width} × {monitor.height}
                      </span>
                    </div>

                    <div>
                      <span className="text-gray-500 dark:text-gray-400">Refresh Rate:</span>
                      <span className="ml-2 font-medium">{monitor.refresh_rate.toFixed(2)} Hz</span>
                    </div>

                    <div>
                      <span className="text-gray-500 dark:text-gray-400">Position:</span>
                      <span className="ml-2 font-medium">
                        ({monitor.x}, {monitor.y})
                      </span>
                    </div>

                    <div>
                      <span className="text-gray-500 dark:text-gray-400">Scale:</span>
                      <span className="ml-2 font-medium">{monitor.scale}×</span>
                    </div>

                    <div>
                      <span className="text-gray-500 dark:text-gray-400">Transform:</span>
                      <span className="ml-2 font-medium">{monitor.transform}</span>
                    </div>

                    <div>
                      <span className="text-gray-500 dark:text-gray-400">Workspace:</span>
                      <span className="ml-2 font-medium">
                        {monitor.active_workspace_name} (#{monitor.active_workspace_id})
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          ))
        )}
      </div>

      <button
        onClick={loadMonitors}
        className="mt-6 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors"
      >
        Refresh
      </button>
    </div>
  );
}
