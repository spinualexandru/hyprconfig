import { Music, Volume2, VolumeOff } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Slider } from "@/components/ui/slider";
import type { AudioStream } from "@/types/audio";

interface AudioStreamCardProps {
	stream: AudioStream;
	onVolumeChange: (volume: number) => void;
	onMuteToggle: () => void;
}

export function AudioStreamCard({
	stream,
	onVolumeChange,
	onMuteToggle,
}: AudioStreamCardProps) {
	const volumePercent = Math.round(stream.volume * 100);

	return (
		<Card className="transition-colors">
			<CardContent className="pt-4">
				<div className="flex items-start gap-3">
					<div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 shrink-0">
						<Music className="h-5 w-5 text-primary" />
					</div>
					<div className="flex-1 min-w-0">
						<div className="font-semibold text-foreground truncate mb-1">
							{stream.app_name}
						</div>
						{stream.media_name && (
							<div className="text-sm text-muted-foreground truncate mb-3">
								{stream.media_name}
							</div>
						)}
						{!stream.media_name && <div className="mb-3" />}
						<div className="flex items-center gap-3">
							<Button
								variant={stream.muted ? "secondary" : "ghost"}
								size="icon"
								className="h-8 w-8 shrink-0"
								onClick={onMuteToggle}
							>
								{stream.muted ? (
									<VolumeOff className="h-4 w-4" />
								) : (
									<Volume2 className="h-4 w-4" />
								)}
							</Button>
							<Slider
								value={[stream.volume]}
								min={0}
								max={1.5}
								step={0.01}
								onValueChange={(value) => onVolumeChange(value[0])}
								className={`flex-1 ${stream.muted ? "opacity-50" : ""}`}
								disabled={stream.muted}
							/>
							<span
								className={`text-sm font-mono w-12 text-right ${volumePercent > 100 ? "text-orange-500" : "text-muted-foreground"}`}
							>
								{volumePercent}%
							</span>
						</div>
					</div>
				</div>
			</CardContent>
		</Card>
	);
}
