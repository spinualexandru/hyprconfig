import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Monitor } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { DisplayCardSkeleton } from "@/components/displays/DisplaySkeleton";

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
  const [cachedMonitors, setCachedMonitors] = useState<MonitorInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [initialLoad, setInitialLoad] = useState(true);
  const [error, setError] = useState<string | null>(null);

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

  // Use cached monitors if loading and cache exists, otherwise use current monitors
  const displayMonitors = loading && cachedMonitors.length > 0 ? cachedMonitors : monitors;

  return (
    <div className="p-6 space-y-6">
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
                  <div className="space-y-1">
                    <p className="text-muted-foreground">Resolution</p>
                    <p className="font-medium">
                      {monitor.width} × {monitor.height}
                    </p>
                  </div>

                  <div className="space-y-1">
                    <p className="text-muted-foreground">Refresh Rate</p>
                    <p className="font-medium">{monitor.refresh_rate.toFixed(2)} Hz</p>
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
    </div>
  );
}
