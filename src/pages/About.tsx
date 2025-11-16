import { invoke } from "@tauri-apps/api/core";
import { RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { SystemInfoSkeleton } from "@/components/about/SystemInfoSkeleton";
import { InfoField } from "@/components/about/InfoField";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";

interface SystemInfo {
	os: string;
	hostname: string;
	kernel: string;
	uptime: string;
	shell: string;
	hyprland_version: string;
	gpus: string[];
	ram_used: string;
	ram_total: string;
	disk_used: string;
	disk_total: string;
	cpu: string;
}

export default function About() {
	const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
	const [cachedSystemInfo, setCachedSystemInfo] = useState<SystemInfo | null>(
		null,
	);
	const [loading, setLoading] = useState(false);
	const [initialLoad, setInitialLoad] = useState(true);
	const [error, setError] = useState<string | null>(null);

	const loadSystemInfo = async () => {
		setLoading(true);
		setError(null);

		invoke<SystemInfo>("get_system_info")
			.then((info) => {
				setSystemInfo(info);
				setCachedSystemInfo(info); // Cache for next time
			})
			.catch((err) => {
				setError(err as string);
				// Keep showing cached data if available
				if (!cachedSystemInfo) {
					setSystemInfo(null);
				}
			})
			.finally(() => {
				setLoading(false);
				setInitialLoad(false);
			});
	};

	useEffect(() => {
		loadSystemInfo();
	}, []);

	return (
		<div className="p-6 space-y-6">
			<div>
				<h1 className="text-3xl font-bold tracking-tight">Hyprconfig</h1>
				<p className="text-muted-foreground mt-2">Version 0.1.0</p>
				<p className="text-muted-foreground mt-2">
					Hyprconfig is not an official Hyprland project and is not affiliated
					with Hyprland in any way.
				</p>
			</div>

			<Card>
				<CardHeader className="flex flex-row items-center justify-between">
					<div>
						<CardTitle>System Information</CardTitle>
						<CardDescription>Details about your system</CardDescription>
					</div>
					<Button
						variant="outline"
						size="icon"
						onClick={loadSystemInfo}
						disabled={loading}
					>
						<RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
					</Button>
				</CardHeader>
				<CardContent>
					{/* Error Display - only show if no cached data */}
					{error && !cachedSystemInfo && (
						<div className="text-sm text-destructive">
							Failed to load system information: {error}
						</div>
					)}

					{/* Use cached data if loading and cache exists, otherwise use current data */}
					{(() => {
						const displayInfo =
							loading && cachedSystemInfo ? cachedSystemInfo : systemInfo;

						// Show skeleton on initial load with no cache
						if ((loading && !cachedSystemInfo) || initialLoad) {
							return <SystemInfoSkeleton />;
						}

						// Show system info if available
						if (displayInfo) {
							return (
								<div className="grid grid-cols-1 md:grid-cols-2 gap-4">
									<InfoField label="OS" value={displayInfo.os} />
									<InfoField label="Hostname" value={displayInfo.hostname} />
									<InfoField label="Kernel" value={displayInfo.kernel} />
									<InfoField label="Uptime" value={displayInfo.uptime} />
									<InfoField label="Shell" value={displayInfo.shell} />
									<InfoField
										label="Hyprland Version"
										value={displayInfo.hyprland_version}
									/>
									<InfoField label="CPU" value={displayInfo.cpu} />
									<InfoField
										label="RAM"
										value={`${displayInfo.ram_used} / ${displayInfo.ram_total}`}
									/>
									<InfoField
										label="Disk Space (Root)"
										value={`${displayInfo.disk_used} / ${displayInfo.disk_total}`}
									/>
									<InfoField
										label={`GPU${displayInfo.gpus.length > 1 ? "s" : ""}`}
										value={displayInfo.gpus}
										className="md:col-span-2"
									/>
								</div>
							);
						}

						return null;
					})()}
				</CardContent>
			</Card>
			<Card>
				<CardHeader>
					<CardTitle>Hyprconfig</CardTitle>
				</CardHeader>
				<CardContent>
					<div>
						<p className="text-sm font-medium">Version</p>
						<p className="text-sm text-muted-foreground">0.1.0</p>
					</div>
					<div>
						<p className="text-sm font-medium">License</p>
						<p className="text-sm text-muted-foreground">MIT License</p>
					</div>
				</CardContent>
			</Card>
		</div>
	);
}
