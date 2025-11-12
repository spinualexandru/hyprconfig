import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Wifi,
  Cable,
  Network as NetworkIcon,
  Info,
  ArrowLeft,
} from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

interface NetworkInterface {
  name: string;
  state: string;
  mac_address: string;
  ip_addresses: string[];
  interface_type: string;
  mtu: string;
  rx_bytes: number;
  tx_bytes: number;
  ssid?: string;
}

export default function Network() {
  const [interfaces, setInterfaces] = useState<NetworkInterface[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedInterface, setSelectedInterface] =
    useState<NetworkInterface | null>(null);

  useEffect(() => {
    loadNetworkInfo();
  }, []);

  const loadNetworkInfo = async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<NetworkInterface[]>("get_network_info");
      setInterfaces(result);
    } catch (err) {
      setError(err as string);
    } finally {
      setLoading(false);
    }
  };

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / k ** i).toFixed(2)} ${sizes[i]}`;
  };

  const getInterfaceIcon = (type: string) => {
    switch (type) {
      case "WiFi":
        return <Wifi className="h-5 w-5 text-primary" />;
      case "Ethernet":
        return <Cable className="h-5 w-5 text-primary" />;
      default:
        return <NetworkIcon className="h-5 w-5 text-primary" />;
    }
  };

  // Show detail view if an interface is selected
  if (selectedInterface) {
    return (
      <div className="p-6 space-y-6">
        <div className="flex items-center gap-4">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setSelectedInterface(null)}
          >
            <ArrowLeft className="h-5 w-5" />
          </Button>
          <div>
            <h1 className="text-3xl font-bold tracking-tight">
              {selectedInterface.ssid
                ? `${selectedInterface.ssid} (${selectedInterface.interface_type})`
                : selectedInterface.interface_type}
            </h1>
            <p className="text-muted-foreground mt-2">
              {selectedInterface.name}
            </p>
          </div>
        </div>

        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>Connection Status</CardTitle>
              <Badge
                variant={
                  selectedInterface.state === "up" ? "default" : "secondary"
                }
                className={
                  selectedInterface.state === "up"
                    ? "bg-green-500 hover:bg-green-600"
                    : ""
                }
              >
                {selectedInterface.state.toUpperCase()}
              </Badge>
            </div>
          </CardHeader>
          <CardContent>
            <div className="space-y-6">
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">Interface Name</p>
                <p className="font-medium">{selectedInterface.name}</p>
              </div>

              {selectedInterface.ssid && (
                <div className="space-y-1">
                  <p className="text-sm text-muted-foreground">
                    Network Name (SSID)
                  </p>
                  <p className="font-medium">{selectedInterface.ssid}</p>
                </div>
              )}

              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">Interface Type</p>
                <p className="font-medium">
                  {selectedInterface.interface_type}
                </p>
              </div>

              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">MAC Address</p>
                <p className="font-medium font-mono text-xs">
                  {selectedInterface.mac_address}
                </p>
              </div>

              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">IP Addresses</p>
                {selectedInterface.ip_addresses.length > 0 ? (
                  <div className="flex flex-wrap gap-2">
                    {selectedInterface.ip_addresses.map((ip, idx) => (
                      <Badge
                        key={idx}
                        variant="outline"
                        className="font-mono text-xs"
                      >
                        {ip}
                      </Badge>
                    ))}
                  </div>
                ) : (
                  <p className="font-medium text-muted-foreground">
                    No IP addresses assigned
                  </p>
                )}
              </div>

              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">
                  MTU (Maximum Transmission Unit)
                </p>
                <p className="font-medium">{selectedInterface.mtu} bytes</p>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1">
                  <p className="text-sm text-muted-foreground">Data Received</p>
                  <p className="font-medium">
                    {formatBytes(selectedInterface.rx_bytes)}
                  </p>
                </div>

                <div className="space-y-1">
                  <p className="text-sm text-muted-foreground">
                    Data Transmitted
                  </p>
                  <p className="font-medium">
                    {formatBytes(selectedInterface.tx_bytes)}
                  </p>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="p-6 space-y-6">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Network</h1>
          <p className="text-muted-foreground mt-2">
            Loading network information...
          </p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-6 space-y-6">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Network</h1>
          <p className="text-muted-foreground mt-2">
            Network settings and configuration
          </p>
        </div>
        <Card className="border-destructive">
          <CardHeader>
            <CardTitle className="text-destructive">
              Error Loading Network Information
            </CardTitle>
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
          <h1 className="text-3xl font-bold tracking-tight">Network</h1>
          <p className="text-muted-foreground mt-2">
            Network settings and configuration
          </p>
        </div>
        <Button onClick={loadNetworkInfo} variant="outline">
          Refresh
        </Button>
      </div>

      {interfaces.length === 0 ? (
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">
              No network interfaces detected.
            </p>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4">
          {interfaces.map((iface) => (
            <Card
              key={iface.name}
              className="cursor-pointer hover:bg-accent/50 transition-colors"
            >
              <CardContent className="pt-0">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3 flex-1">
                    <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
                      {getInterfaceIcon(iface.interface_type)}
                    </div>
                    <div className="flex-1">
                      <div className="font-semibold">
                        {iface.ssid
                          ? `${iface.ssid} (${iface.interface_type})`
                          : iface.interface_type}
                      </div>
                      <div className="text-sm text-muted-foreground">
                        {iface.name}
                      </div>
                      {iface.ip_addresses.length > 0 && (
                        <div className="text-xs text-muted-foreground font-mono mt-1">
                          {iface.ip_addresses[0]}
                        </div>
                      )}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge
                      variant={iface.state === "up" ? "default" : "secondary"}
                      className={
                        iface.state === "up"
                          ? "bg-green-500 hover:bg-green-600"
                          : ""
                      }
                    >
                      {iface.state.toUpperCase()}
                    </Badge>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => setSelectedInterface(iface)}
                    >
                      <Info className="h-5 w-5" />
                    </Button>
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
