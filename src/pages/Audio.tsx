import { invoke } from "@tauri-apps/api/core";
import { Mic, RefreshCw, Volume2 } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { AudioDeviceCard } from "@/components/audio/AudioDeviceCard";
import { AudioSectionSkeleton } from "@/components/audio/AudioSkeletons";
import { AudioStreamCard } from "@/components/audio/AudioStreamCard";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { AudioDevice, AudioState, AudioStream } from "@/types/audio";

export default function Audio() {
	const [audioState, setAudioState] = useState<AudioState | null>(null);
	const [cachedAudioState, setCachedAudioState] = useState<AudioState | null>(
		null,
	);
	const [loading, setLoading] = useState(false);
	const [initialLoad, setInitialLoad] = useState(true);
	const [error, setError] = useState<string | null>(null);

	// Debounce timers for volume changes
	const volumeTimers = useRef<Map<number, ReturnType<typeof setTimeout>>>(
		new Map(),
	);

	const loadAudioState = useCallback(async () => {
		setLoading(true);
		setError(null);

		invoke<AudioState>("get_audio_state")
			.then((result) => {
				setAudioState(result);
				setCachedAudioState(result);
			})
			.catch((err) => {
				setError(err as string);
				if (!cachedAudioState) {
					setAudioState(null);
				}
			})
			.finally(() => {
				setLoading(false);
				setInitialLoad(false);
			});
	}, [cachedAudioState]);

	useEffect(() => {
		loadAudioState();
	}, [loadAudioState]);

	// Cleanup timers on unmount
	useEffect(() => {
		return () => {
			for (const timer of volumeTimers.current.values()) {
				clearTimeout(timer);
			}
		};
	}, []);

	const handleVolumeChange = (nodeId: number, volume: number) => {
		// Update local state immediately for responsiveness
		if (audioState) {
			const updateDevice = (devices: AudioDevice[]) =>
				devices.map((d) => (d.id === nodeId ? { ...d, volume } : d));
			const updateStream = (streams: AudioStream[]) =>
				streams.map((s) => (s.id === nodeId ? { ...s, volume } : s));

			setAudioState({
				...audioState,
				sinks: updateDevice(audioState.sinks),
				sources: updateDevice(audioState.sources),
				streams: updateStream(audioState.streams),
			});
		}

		// Debounce the actual API call
		const existingTimer = volumeTimers.current.get(nodeId);
		if (existingTimer) {
			clearTimeout(existingTimer);
		}

		const timer = setTimeout(() => {
			invoke("set_volume", { nodeId, volume }).catch((err) => {
				console.error("Failed to set volume:", err);
			});
			volumeTimers.current.delete(nodeId);
		}, 100);

		volumeTimers.current.set(nodeId, timer);
	};

	const handleMuteToggle = async (nodeId: number, currentMuted: boolean) => {
		const newMuted = !currentMuted;

		// Update local state immediately
		if (audioState) {
			const updateDevice = (devices: AudioDevice[]) =>
				devices.map((d) => (d.id === nodeId ? { ...d, muted: newMuted } : d));
			const updateStream = (streams: AudioStream[]) =>
				streams.map((s) => (s.id === nodeId ? { ...s, muted: newMuted } : s));

			setAudioState({
				...audioState,
				sinks: updateDevice(audioState.sinks),
				sources: updateDevice(audioState.sources),
				streams: updateStream(audioState.streams),
			});
		}

		try {
			await invoke("set_mute", { nodeId, muted: newMuted });
		} catch (err) {
			console.error("Failed to toggle mute:", err);
			// Revert on error
			loadAudioState();
		}
	};

	const handleSetDefault = async (deviceId: number) => {
		try {
			await invoke("set_default_device", { deviceId });
			// Reload to get updated default status
			await loadAudioState();
		} catch (err) {
			console.error("Failed to set default device:", err);
		}
	};

	// Display data - use cached if loading and cache exists
	const displayState =
		loading && cachedAudioState ? cachedAudioState : audioState;
	const showSkeleton = (initialLoad || (loading && !cachedAudioState)) && !error;

	return (
		<div className="p-6 space-y-6">
			<div className="flex items-center justify-between">
				<div>
					<h1 className="text-3xl font-bold tracking-tight text-foreground">
						Audio
					</h1>
					<p className="text-muted-foreground mt-2">
						Manage audio devices and application volumes
					</p>
				</div>
				<Button onClick={loadAudioState} variant="outline" disabled={loading}>
					<RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
					<span className="ml-2">Refresh</span>
				</Button>
			</div>

			{/* Error Display */}
			{error && (
				<Card className="border-destructive">
					<CardHeader>
						<CardTitle className="text-destructive">
							Error Loading Audio Information
						</CardTitle>
					</CardHeader>
					<CardContent>
						<p className="text-sm text-muted-foreground">{error}</p>
						{error.includes("wpctl not found") && (
							<p className="text-sm text-muted-foreground mt-2">
								Please install WirePlumber to manage PipeWire audio devices.
							</p>
						)}
					</CardContent>
				</Card>
			)}

			{/* Output Devices (Sinks) Section */}
			<div className="space-y-4">
				<div className="flex items-center gap-2">
					<Volume2 className="h-5 w-5 text-muted-foreground" />
					<h2 className="text-xl font-semibold text-foreground">
						Output Devices
					</h2>
				</div>
				<div className="grid gap-4">
					{showSkeleton ? (
						<AudioSectionSkeleton count={1} />
					) : displayState && displayState.sinks.length > 0 ? (
						displayState.sinks.map((device) => (
							<AudioDeviceCard
								key={device.id}
								device={device}
								onVolumeChange={(vol) => handleVolumeChange(device.id, vol)}
								onMuteToggle={() => handleMuteToggle(device.id, device.muted)}
								onSetDefault={() => handleSetDefault(device.id)}
							/>
						))
					) : (
						!loading &&
						!error && (
							<Card>
								<CardContent className="pt-6">
									<p className="text-sm text-muted-foreground">
										No output devices detected.
									</p>
								</CardContent>
							</Card>
						)
					)}
				</div>
			</div>

			{/* Input Devices (Sources) Section */}
			<div className="space-y-4">
				<div className="flex items-center gap-2">
					<Mic className="h-5 w-5 text-muted-foreground" />
					<h2 className="text-xl font-semibold text-foreground">
						Input Devices
					</h2>
				</div>
				<div className="grid gap-4">
					{showSkeleton ? (
						<AudioSectionSkeleton count={1} />
					) : displayState && displayState.sources.length > 0 ? (
						displayState.sources.map((device) => (
							<AudioDeviceCard
								key={device.id}
								device={device}
								onVolumeChange={(vol) => handleVolumeChange(device.id, vol)}
								onMuteToggle={() => handleMuteToggle(device.id, device.muted)}
								onSetDefault={() => handleSetDefault(device.id)}
							/>
						))
					) : (
						!loading &&
						!error && (
							<Card>
								<CardContent className="pt-6">
									<p className="text-sm text-muted-foreground">
										No input devices detected.
									</p>
								</CardContent>
							</Card>
						)
					)}
				</div>
			</div>

			{/* Application Streams Section */}
			{((displayState && displayState.streams.length > 0) || showSkeleton) && (
				<div className="space-y-4">
					<h2 className="text-xl font-semibold text-foreground">
						Application Audio
					</h2>
					<div className="grid gap-4">
						{showSkeleton ? (
							<AudioSectionSkeleton count={2} />
						) : (
							displayState?.streams.map((stream) => (
								<AudioStreamCard
									key={stream.id}
									stream={stream}
									onVolumeChange={(vol) => handleVolumeChange(stream.id, vol)}
									onMuteToggle={() =>
										handleMuteToggle(stream.id, stream.muted)
									}
								/>
							))
						)}
					</div>
				</div>
			)}

			{/* Empty state for streams when no apps are playing */}
			{!showSkeleton &&
				!error &&
				displayState &&
				displayState.streams.length === 0 &&
				(displayState.sinks.length > 0 || displayState.sources.length > 0) && (
					<div className="space-y-4">
						<h2 className="text-xl font-semibold text-foreground">
							Application Audio
						</h2>
						<Card>
							<CardContent className="pt-6">
								<p className="text-sm text-muted-foreground">
									No applications are currently playing audio.
								</p>
							</CardContent>
						</Card>
					</div>
				)}
		</div>
	);
}
