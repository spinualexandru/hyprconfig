import { Mic, MicOff, Speaker, Volume2, VolumeOff } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Slider } from "@/components/ui/slider";
import type { AudioDevice } from "@/types/audio";

interface AudioDeviceCardProps {
	device: AudioDevice;
	onVolumeChange: (volume: number) => void;
	onMuteToggle: () => void;
	onSetDefault: () => void;
}

export function AudioDeviceCard({
	device,
	onVolumeChange,
	onMuteToggle,
	onSetDefault,
}: AudioDeviceCardProps) {
	const isSink = device.device_type === "sink";
	const volumePercent = Math.round(device.volume * 100);

	const getDeviceIcon = () => {
		if (isSink) {
			return device.muted ? (
				<VolumeOff className="h-5 w-5 text-muted-foreground" />
			) : (
				<Volume2 className="h-5 w-5 text-primary" />
			);
		}
		return device.muted ? (
			<MicOff className="h-5 w-5 text-muted-foreground" />
		) : (
			<Mic className="h-5 w-5 text-primary" />
		);
	};

	const getMuteIcon = () => {
		if (isSink) {
			return device.muted ? (
				<VolumeOff className="h-4 w-4" />
			) : (
				<Volume2 className="h-4 w-4" />
			);
		}
		return device.muted ? (
			<MicOff className="h-4 w-4" />
		) : (
			<Mic className="h-4 w-4" />
		);
	};

	return (
		<Card
			className={`transition-colors ${device.is_default ? "border-primary" : ""}`}
		>
			<CardContent className="pt-4">
				<div className="flex items-start gap-3">
					<div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 shrink-0">
						{getDeviceIcon()}
					</div>
					<div className="flex-1 min-w-0">
						<div className="flex items-center gap-2 mb-1">
							<span className="font-semibold text-foreground truncate">
								{device.description}
							</span>
							{device.is_default && (
								<Badge variant="default" className="shrink-0">
									Default
								</Badge>
							)}
						</div>
						<div className="text-sm text-muted-foreground mb-3">
							{isSink ? "Output" : "Input"} Device
						</div>
						<div className="flex items-center gap-3">
							<Button
								variant={device.muted ? "secondary" : "ghost"}
								size="icon"
								className="h-8 w-8 shrink-0"
								onClick={onMuteToggle}
							>
								{getMuteIcon()}
							</Button>
							<Slider
								value={[device.volume]}
								min={0}
								max={1.5}
								step={0.01}
								onValueChange={(value) => onVolumeChange(value[0])}
								className={`flex-1 ${device.muted ? "opacity-50" : ""}`}
								disabled={device.muted}
							/>
							<span
								className={`text-sm font-mono w-12 text-right ${volumePercent > 100 ? "text-orange-500" : "text-muted-foreground"}`}
							>
								{volumePercent}%
							</span>
						</div>
					</div>
				</div>
				{!device.is_default && (
					<div className="mt-3 pt-3 border-t">
						<Button
							variant="outline"
							size="sm"
							onClick={onSetDefault}
							className="w-full"
						>
							<Speaker className="h-4 w-4 mr-2" />
							Set as Default
						</Button>
					</div>
				)}
			</CardContent>
		</Card>
	);
}
