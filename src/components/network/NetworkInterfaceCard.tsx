import { Cable, Info, Network as NetworkIcon, Wifi } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";

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

interface NetworkInterfaceCardProps {
	interface: NetworkInterface;
	onInfoClick: (iface: NetworkInterface) => void;
}

export function NetworkInterfaceCard({
	interface: iface,
	onInfoClick,
}: NetworkInterfaceCardProps) {
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

	return (
		<Card className="cursor-pointer hover:bg-accent/50 transition-colors">
			<CardContent>
				<div className="flex items-center justify-between">
					<div className="flex items-center gap-3 flex-1">
						<div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
							{getInterfaceIcon(iface.interface_type)}
						</div>
						<div className="flex-1">
							<div className="font-semibold text-foreground">
								{iface.ssid
									? `${iface.ssid} (${iface.interface_type})`
									: iface.interface_type}
							</div>
							<div className="text-sm text-muted-foreground">{iface.name}</div>
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
								iface.state === "up" ? "bg-green-500 hover:bg-green-600" : ""
							}
						>
							{iface.state.toUpperCase()}
						</Badge>
						<Button
							variant="ghost"
							size="icon"
							onClick={() => onInfoClick(iface)}
						>
							<Info className="h-5 w-5" />
						</Button>
					</div>
				</div>
			</CardContent>
		</Card>
	);
}
