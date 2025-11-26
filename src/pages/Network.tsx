import { invoke } from "@tauri-apps/api/core";
import { RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { NetworkInterfaceCard } from "@/components/network/NetworkInterfaceCard";
import { NetworkInterfaceDetail } from "@/components/network/NetworkInterfaceDetail";
import {
	NetworkCardSkeleton,
	WifiCardSkeleton,
} from "@/components/network/NetworkSkeletons";
import { WifiNetworkCard } from "@/components/network/WifiNetworkCard";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

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

interface WifiNetwork {
	ssid: string;
	signal_strength: number;
	security: string;
	connected: boolean;
	bssid: string;
	frequency: string;
}

export default function Network() {
	const [interfaces, setInterfaces] = useState<NetworkInterface[]>([]);
	const [wifiNetworks, setWifiNetworks] = useState<WifiNetwork[]>([]);
	const [cachedWifiNetworks, setCachedWifiNetworks] = useState<WifiNetwork[]>(
		[],
	);
	const [loadingInterfaces, setLoadingInterfaces] = useState(false);
	const [loadingWifi, setLoadingWifi] = useState(false);
	const [initialLoad, setInitialLoad] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [selectedInterface, setSelectedInterface] =
		useState<NetworkInterface | null>(null);

	useEffect(() => {
		loadNetworkInfo();
	}, []);

	const scanWifiNetworks = async () => {
		// Load WiFi networks independently (takes longer due to scan)
		// Show cached networks immediately if available
		setLoadingWifi(true);
		invoke<WifiNetwork[]>("scan_wifi_networks")
			.then((result) => {
				setWifiNetworks(result);
				setCachedWifiNetworks(result); // Cache for next time
			})
			.catch(() => {
				// Silently fail for WiFi scan (may not be available on all systems)
				if (cachedWifiNetworks.length === 0) {
					setWifiNetworks([]);
				}
			})
			.finally(() => {
				setLoadingWifi(false);
				setInitialLoad(false);
			});
	};

	const loadNetworkInfo = async () => {
		setError(null);

		// Load interfaces independently
		setLoadingInterfaces(true);
		invoke<NetworkInterface[]>("get_network_info")
			.then((result) => {
				setInterfaces(result);
			})
			.catch((err) => {
				setError(err as string);
			})
			.finally(() => {
				setLoadingInterfaces(false);
				setInitialLoad(false);
			});

		// Also scan WiFi networks
		scanWifiNetworks();
	};

	// Show detail view if an interface is selected
	if (selectedInterface) {
		return (
			<NetworkInterfaceDetail
				interface={selectedInterface}
				onBack={() => setSelectedInterface(null)}
			/>
		);
	}

	// Use cached networks if loading and cache exists, otherwise use current networks
	const displayWifiNetworks =
		loadingWifi && cachedWifiNetworks.length > 0
			? cachedWifiNetworks
			: wifiNetworks;
	const connectedWifi = displayWifiNetworks.filter((net) => net.connected);
	const availableWifi = displayWifiNetworks.filter((net) => !net.connected);

	// Show WiFi loading spinner only when refreshing with cached data
	const showWifiRefreshSpinner = loadingWifi && cachedWifiNetworks.length > 0;

	return (
		<div className="p-6 space-y-6">
			<div className="flex items-center justify-between">
				<div>
					<h1 className="text-3xl font-bold tracking-tight text-foreground">Network</h1>
					<p className="text-muted-foreground mt-2">
						Network settings and configuration
					</p>
				</div>
				<Button
					onClick={loadNetworkInfo}
					variant="outline"
					disabled={loadingInterfaces || loadingWifi}
				>
					Refresh
				</Button>
			</div>

			{/* Error Display */}
			{error && (
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
			)}

			{/* Network Interfaces Section */}
			{(loadingInterfaces || interfaces.length > 0 || initialLoad) && (
				<div className="space-y-4">
					<h2 className="text-xl font-semibold text-foreground">Network Interfaces</h2>
					<div className="grid gap-4">
						{(loadingInterfaces && interfaces.length === 0) || initialLoad ? (
							<>
								<NetworkCardSkeleton />
								<NetworkCardSkeleton />
							</>
						) : (
							interfaces.map((iface) => (
								<NetworkInterfaceCard
									key={iface.name}
									interface={iface}
									onInfoClick={setSelectedInterface}
								/>
							))
						)}
					</div>
				</div>
			)}

			{/* Connected WiFi Networks Section */}
			{((loadingWifi && cachedWifiNetworks.length === 0) ||
				connectedWifi.length > 0 ||
				initialLoad) && (
				<div className="space-y-4">
					<h2 className="text-xl font-semibold text-foreground">Connected</h2>
					<div className="grid gap-4">
						{(loadingWifi && cachedWifiNetworks.length === 0) || initialLoad ? (
							<WifiCardSkeleton />
						) : (
							connectedWifi.map((network) => (
								<WifiNetworkCard key={network.bssid} network={network} />
							))
						)}
					</div>
				</div>
			)}

			{/* Available WiFi Networks Section */}
			{((loadingWifi && cachedWifiNetworks.length === 0) ||
				availableWifi.length > 0 ||
				initialLoad) && (
				<div className="space-y-4">
					<div className="flex items-center justify-between">
						<h2 className="text-xl font-semibold text-foreground">Available Networks</h2>
						<Button
							variant="ghost"
							size="icon"
							onClick={scanWifiNetworks}
							disabled={loadingWifi}
						>
							<RefreshCw
								className={`h-4 w-4 ${loadingWifi ? "animate-spin" : ""}`}
							/>
						</Button>
					</div>
					<div className="grid gap-4">
						{(loadingWifi && cachedWifiNetworks.length === 0) || initialLoad ? (
							<>
								<WifiCardSkeleton />
								<WifiCardSkeleton />
								<WifiCardSkeleton />
							</>
						) : (
							availableWifi.map((network) => (
								<WifiNetworkCard key={network.bssid} network={network} />
							))
						)}
					</div>
				</div>
			)}

			{!loadingInterfaces &&
				!loadingWifi &&
				!initialLoad &&
				interfaces.length === 0 &&
				wifiNetworks.length === 0 && (
					<Card>
						<CardContent className="pt-6">
							<p className="text-sm text-muted-foreground">
								No network interfaces or WiFi networks detected.
							</p>
						</CardContent>
					</Card>
				)}
		</div>
	);
}
