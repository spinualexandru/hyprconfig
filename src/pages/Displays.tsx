import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Monitor } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { DisplayCardSkeleton } from "@/components/displays/DisplaySkeleton";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

interface DisplayMode {
  width: number;
  height: number;
  refresh_rate: number;
}

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
  available_modes: DisplayMode[];
}

interface MonitorSettings {
  [monitorId: number]: {
    resolution: string; // "widthxheight"
    refreshRate: string; // "rate"
  };
}

export default function Displays() {
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [cachedMonitors, setCachedMonitors] = useState<MonitorInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [initialLoad, setInitialLoad] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [monitorSettings, setMonitorSettings] = useState<MonitorSettings>({});
  const [hasChanges, setHasChanges] = useState(false);

  useEffect(() => {
    loadMonitors();
  }, []);

  const loadMonitors = async () => {
    setLoading(true);
    setError(null);

    invoke<MonitorInfo[]>("get_monitors")
      .then((result) => {
        setMonitors(result);
        setCachedMonitors(result); // Cache for next time

        // Initialize monitor settings with current values
        const initialSettings: MonitorSettings = {};
        result.forEach((monitor) => {
          initialSettings[monitor.id] = {
            resolution: `${monitor.width}x${monitor.height}`,
            refreshRate: monitor.refresh_rate.toFixed(2),
          };
        });
        setMonitorSettings(initialSettings);
        setHasChanges(false);
      })
      .catch((err) => {
        setError(err as string);
        // Keep showing cached data if available
        if (cachedMonitors.length === 0) {
          setMonitors([]);
        }
      })
      .finally(() => {
        setLoading(false);
        setInitialLoad(false);
      });
  };

  const handleResolutionChange = (monitorId: number, resolution: string) => {
    // Find the monitor to get available refresh rates for the new resolution
    const monitor = displayMonitors.find((m) => m.id === monitorId);
    if (monitor) {
      const availableRatesForResolution = monitor.available_modes
        .filter((mode) => `${mode.width}x${mode.height}` === resolution)
        .map((mode) => mode.refresh_rate.toFixed(2));

      // Check if current refresh rate is available for new resolution
      const currentRefreshRate = monitorSettings[monitorId]?.refreshRate;
      const newRefreshRate = availableRatesForResolution.includes(currentRefreshRate)
        ? currentRefreshRate
        : availableRatesForResolution[0] || currentRefreshRate;

      setMonitorSettings((prev) => ({
        ...prev,
        [monitorId]: {
          resolution,
          refreshRate: newRefreshRate,
        },
      }));
    }
    setHasChanges(true);
  };

  const handleRefreshRateChange = (monitorId: number, refreshRate: string) => {
    setMonitorSettings((prev) => ({
      ...prev,
      [monitorId]: {
        ...prev[monitorId],
        refreshRate,
      },
    }));
    setHasChanges(true);
  };

  const handleApply = () => {
    // TODO: Apply the changes (implement later)
    console.log("Applying changes:", monitorSettings);
    setHasChanges(false);
  };

  const handleCancel = () => {
    // Reset to current monitor values
    const resetSettings: MonitorSettings = {};
    displayMonitors.forEach((monitor) => {
      resetSettings[monitor.id] = {
        resolution: `${monitor.width}x${monitor.height}`,
        refreshRate: monitor.refresh_rate.toFixed(2),
      };
    });
    setMonitorSettings(resetSettings);
    setHasChanges(false);
  };

  // Use cached monitors if loading and cache exists, otherwise use current monitors
  const displayMonitors = loading && cachedMonitors.length > 0 ? cachedMonitors : monitors;

  return (
    <div className={`p-6 space-y-6 ${hasChanges ? 'pb-24' : ''}`}>
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Displays</h1>
          <p className="text-muted-foreground mt-2">
            Configure your display settings
          </p>
        </div>
        <Button onClick={loadMonitors} variant="outline" disabled={loading}>
          Refresh
        </Button>
      </div>

      {/* Error Display */}
      {error && cachedMonitors.length === 0 && (
        <Card className="border-destructive">
          <CardHeader>
            <CardTitle className="text-destructive">Error Loading Monitors</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground">{error}</p>
          </CardContent>
        </Card>
      )}

      {/* Monitors Display */}
      {(loading && cachedMonitors.length === 0) || initialLoad ? (
        <div className="grid gap-4">
          <DisplayCardSkeleton />
          <DisplayCardSkeleton />
        </div>
      ) : displayMonitors.length === 0 ? (
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">No monitors detected.</p>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4">
          {displayMonitors.map((monitor) => (
            <Card key={monitor.id}>
              <CardHeader>
                <div className="flex items-center gap-3">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
                    <Monitor className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <CardTitle>{monitor.name}</CardTitle>
                    <CardDescription>{monitor.description}</CardDescription>
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div className="space-y-2">
                    <p className="text-muted-foreground">Resolution</p>
                    <Select
                      value={monitorSettings[monitor.id]?.resolution || `${monitor.width}x${monitor.height}`}
                      onValueChange={(value) => handleResolutionChange(monitor.id, value)}
                    >
                      <SelectTrigger className="w-full">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {/* Get unique resolutions from available modes */}
                        {Array.from(
                          new Set(
                            monitor.available_modes.map(
                              (mode) => `${mode.width}x${mode.height}`
                            )
                          )
                        ).map((resolution) => (
                          <SelectItem key={resolution} value={resolution}>
                            {resolution.replace("x", " × ")}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>

                  <div className="space-y-2">
                    <p className="text-muted-foreground">Refresh Rate</p>
                    <Select
                      value={monitorSettings[monitor.id]?.refreshRate || monitor.refresh_rate.toFixed(2)}
                      onValueChange={(value) => handleRefreshRateChange(monitor.id, value)}
                    >
                      <SelectTrigger className="w-full">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {/* Get refresh rates for the selected resolution */}
                        {monitor.available_modes
                          .filter((mode) => {
                            const selectedResolution = monitorSettings[monitor.id]?.resolution || `${monitor.width}x${monitor.height}`;
                            return `${mode.width}x${mode.height}` === selectedResolution;
                          })
                          .map((mode) => {
                            const rate = mode.refresh_rate.toFixed(2);
                            return (
                              <SelectItem key={rate} value={rate}>
                                {rate} Hz
                              </SelectItem>
                            );
                          })}
                      </SelectContent>
                    </Select>
                  </div>

                  <div className="space-y-1">
                    <p className="text-muted-foreground">Position</p>
                    <p className="font-medium">
                      ({monitor.x}, {monitor.y})
                    </p>
                  </div>

                  <div className="space-y-1">
                    <p className="text-muted-foreground">Scale</p>
                    <p className="font-medium">{monitor.scale}×</p>
                  </div>

                  <div className="space-y-1">
                    <p className="text-muted-foreground">Transform</p>
                    <p className="font-medium">{monitor.transform}</p>
                  </div>

                  <div className="space-y-1">
                    <p className="text-muted-foreground">Workspace</p>
                    <p className="font-medium">
                      {monitor.active_workspace_name} (#{monitor.active_workspace_id})
                    </p>
                  </div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Apply/Cancel floating bar - show when changes are made */}
      {hasChanges && !loading && displayMonitors.length > 0 && (
        <div className="fixed bottom-0 left-0 right-0 z-50 border-t bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
          <div className="container flex h-16 items-center justify-between px-6">
            <p className="text-sm text-muted-foreground">
              You have unsaved changes to your display settings.
            </p>
            <div className="flex gap-2">
              <Button variant="outline" onClick={handleCancel}>
                Cancel
              </Button>
              <Button onClick={handleApply}>
                Apply Changes
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
