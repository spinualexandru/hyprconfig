import { ArrowLeft } from "lucide-react";
import { Badge } from "@/components/ui/badge";
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

interface NetworkInterfaceDetailProps {
	interface: NetworkInterface;
	onBack: () => void;
}

export function NetworkInterfaceDetail({
	interface: selectedInterface,
	onBack,
}: NetworkInterfaceDetailProps) {
	const formatBytes = (bytes: number): string => {
		if (bytes === 0) return "0 B";
		const k = 1024;
		const sizes = ["B", "KB", "MB", "GB", "TB"];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return `${(bytes / k ** i).toFixed(2)} ${sizes[i]}`;
	};

	return (
		<div className="p-6 space-y-6">
			<div className="flex items-center gap-4">
				<Button variant="ghost" size="icon" onClick={onBack}>
					<ArrowLeft className="h-5 w-5" />
				</Button>
				<div>
					<h1 className="text-3xl font-bold tracking-tight">
						{selectedInterface.ssid
							? `${selectedInterface.ssid} (${selectedInterface.interface_type})`
							: selectedInterface.interface_type}
					</h1>
					<p className="text-muted-foreground mt-2">{selectedInterface.name}</p>
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
							<p className="font-medium">{selectedInterface.interface_type}</p>
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
