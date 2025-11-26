import { Wifi } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";

interface WifiNetwork {
	ssid: string;
	signal_strength: number;
	security: string;
	connected: boolean;
	bssid: string;
	frequency: string;
}

interface WifiNetworkCardProps {
	network: WifiNetwork;
}

export function WifiNetworkCard({ network }: WifiNetworkCardProps) {
	const getSignalStrengthText = (strength: number) => {
		if (strength >= 75) return "Excellent";
		if (strength >= 50) return "Good";
		if (strength >= 25) return "Fair";
		return "Weak";
	};

	const getSignalStrengthColor = (strength: number) => {
		if (strength >= 75) return "text-green-500";
		if (strength >= 50) return "text-yellow-500";
		if (strength >= 25) return "text-orange-500";
		return "text-red-500";
	};

	return (
		<Card
			className={
				network.connected ? "" : "hover:bg-accent/50 transition-colors"
			}
		>
			<CardContent>
				<div className="flex items-center justify-between">
					<div className="flex items-center gap-3 flex-1">
						<div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
							<Wifi className="h-5 w-5 text-primary" />
						</div>
						<div className="flex-1">
							<div className="font-semibold text-foreground">{network.ssid}</div>
							<div className="text-sm text-muted-foreground">
								{network.security} • {network.frequency}
							</div>
						</div>
					</div>
					<div className="flex items-center gap-2">
						<div className="text-right">
							<div
								className={`text-sm font-medium ${getSignalStrengthColor(network.signal_strength)}`}
							>
								{network.signal_strength}%
							</div>
							<div className="text-xs text-muted-foreground">
								{getSignalStrengthText(network.signal_strength)}
							</div>
						</div>
						{network.connected && (
							<Badge className="bg-green-500 hover:bg-green-600">
								CONNECTED
							</Badge>
						)}
					</div>
				</div>
			</CardContent>
		</Card>
	);
}
