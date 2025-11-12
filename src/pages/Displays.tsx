import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Monitor } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

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
      <div className="p-6 space-y-6">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Displays</h1>
          <p className="text-muted-foreground mt-2">Loading monitor information...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-6 space-y-6">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Displays</h1>
          <p className="text-muted-foreground mt-2">Configure your display settings</p>
        </div>
        <Card className="border-destructive">
          <CardHeader>
            <CardTitle className="text-destructive">Error Loading Monitors</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground">{error}</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Displays</h1>
          <p className="text-muted-foreground mt-2">
            Configure your display settings
          </p>
        </div>
        <Button onClick={loadMonitors} variant="outline">
          Refresh
        </Button>
      </div>

      {monitors.length === 0 ? (
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">No monitors detected.</p>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4">
          {monitors.map((monitor) => (
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
