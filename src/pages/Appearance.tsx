import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import { FileCode, Image as ImageIcon, Palette, RefreshCw, Upload } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { reloadTheme, ensureMatugenTemplate } from "@/lib/theme-loader";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";

interface Wallpaper {
	monitor: string;
	path: string;
	fit_mode: string;
}

interface HyprpaperConfig {
	wallpapers: Wallpaper[];
}

export default function Appearance() {
	const [config, setConfig] = useState<HyprpaperConfig | null>(null);
	const [loading, setLoading] = useState(false);
	const [initialLoad, setInitialLoad] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [updating, setUpdating] = useState(false);
	const [matugenExists, setMatugenExists] = useState(false);
	const [syncMatugen, setSyncMatugen] = useState(false);
	const [matugenLightMode, setMatugenLightMode] = useState(false);
	const [generatorType, setGeneratorType] = useState("scheme-tonal-spot");

	const loadConfig = async () => {
		setLoading(true);
		setError(null);

		try {
			const hyprpaperConfig = await invoke<HyprpaperConfig>(
				"get_hyprpaper_config",
			);
			setConfig(hyprpaperConfig);
		} catch (err) {
			setError(err as string);
			setConfig(null);
		} finally {
			setLoading(false);
			setInitialLoad(false);
		}
	};

	useEffect(() => {
		loadConfig();
	}, []);

	useEffect(() => {
		const checkMatugen = async () => {
			try {
				const exists = await invoke<boolean>("tool_exists", { name: "matugen" });
				setMatugenExists(exists);
			} catch {
				setMatugenExists(false);
			}
		};
		checkMatugen();
	}, []);

	useEffect(() => {
		const loadPreferences = async () => {
			try {
				const prefs = await invoke<{
					matugen: {
						enable: boolean;
						light_mode: boolean;
						generator_type: string;
					};
				}>("get_preferences");
				setSyncMatugen(prefs.matugen.enable);
				setMatugenLightMode(prefs.matugen.light_mode);
				setGeneratorType(prefs.matugen.generator_type);
			} catch (err) {
				console.error("Failed to load preferences:", err);
			}
		};
		loadPreferences();
	}, []);

	const handleChangeWallpaper = async () => {
		try {
			// Get current wallpaper directory to use as default path
			let defaultPath: string | undefined;
			const current = getCurrentWallpaper();
			if (current?.path) {
				const lastSlash = Math.max(
					current.path.lastIndexOf('/'),
					current.path.lastIndexOf('\\')
				);
				if (lastSlash !== -1) {
					defaultPath = current.path.substring(0, lastSlash);
				}
			}

			const selected = await open({
				multiple: false,
				directory: false,
				defaultPath,
				filters: [
					{
						name: "Images",
						extensions: ["jpg", "jpeg", "png", "gif", "webp", "bmp"],
					},
				],
			});

			if (selected) {
				setUpdating(true);

				// Replace all existing wallpapers with the new one
				await invoke("replace_wallpaper", { monitor: "", path: selected });

				// Run matugen if enabled
				if (syncMatugen) {
					try {
						await invoke("run_matugen", {
							imagePath: selected,
							lightMode: matugenLightMode,
							generatorType: generatorType,
						});
						// Reload theme to pick up matugen-generated colors
						await reloadTheme();
						toast.success("Wallpaper updated and Matugen synced");
					} catch (matugenErr) {
						toast.warning(`Wallpaper updated but Matugen failed: ${matugenErr}`);
					}
				} else {
					toast.success("Wallpaper updated successfully");
				}

				await loadConfig();
			}
		} catch (err) {
			toast.error(`Failed to update wallpaper: ${err}`);
		} finally {
			setUpdating(false);
		}
	};

	const handleSyncMatugenChange = async (checked: boolean) => {
		setSyncMatugen(checked);
		try {
			await invoke("update_matugen_preferences", {
				enable: checked,
				lightMode: matugenLightMode,
				generatorType: generatorType,
			});
		} catch (err) {
			toast.error(`Failed to save preference: ${err}`);
		}
	};

	const handleLightModeChange = async (checked: boolean) => {
		setMatugenLightMode(checked);
		try {
			await invoke("update_matugen_preferences", {
				enable: syncMatugen,
				lightMode: checked,
				generatorType: generatorType,
			});
		} catch (err) {
			toast.error(`Failed to save preference: ${err}`);
		}
	};

	const handleGeneratorTypeChange = async (value: string) => {
		setGeneratorType(value);
		try {
			await invoke("update_matugen_preferences", {
				enable: syncMatugen,
				lightMode: matugenLightMode,
				generatorType: value,
			});
		} catch (err) {
			toast.error(`Failed to save preference: ${err}`);
		}
	};

	const getCurrentWallpaper = () => {
		if (!config || config.wallpapers.length === 0) return null;
		// Get the last wallpaper (most recent)
		return config.wallpapers[config.wallpapers.length - 1];
	};

	const currentWallpaper = getCurrentWallpaper();

	return (
		<div className="p-6 space-y-6">
			<div>
				<h1 className="text-3xl font-bold tracking-tight text-foreground">Appearance</h1>
				<p className="text-muted-foreground mt-2">
					Customize your wallpaper and appearance settings
				</p>
			</div>

			<Card>
				<CardHeader className="flex flex-row items-center justify-between">
					<div>
						<CardTitle>Wallpaper</CardTitle>
						<CardDescription>
							Current wallpaper from hyprpaper configuration
						</CardDescription>
					</div>
					<div className="flex gap-2 items-center">
						<Button
							variant="outline"
							size="icon"
							onClick={loadConfig}
							disabled={loading}
						>
							<RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
						</Button>
						<Button
							onClick={handleChangeWallpaper}
							disabled={updating || loading}
						>
							<Upload className="h-4 w-4 mr-2" />
							Change Wallpaper
						</Button>
					</div>
				</CardHeader>
				<CardContent>
					{error && (
						<div className="text-sm text-destructive mb-4">
							Failed to load wallpaper configuration: {error}
						</div>
					)}

					{initialLoad && loading ? (
						<div className="space-y-4">
							<Skeleton className="w-full h-[400px] rounded-lg" />
							<Skeleton className="w-1/2 h-4" />
						</div>
					) : currentWallpaper ? (
						<div className="space-y-4">
							<div className="relative w-full h-[400px] rounded-lg overflow-hidden bg-muted">
								<img
									src={convertFileSrc(currentWallpaper.path)}
									alt="Current wallpaper"
									className="w-full h-full object-cover"
									onError={(e) => {
										// Fallback if image fails to load
										e.currentTarget.style.display = "none";
										e.currentTarget.parentElement?.classList.add(
											"flex",
											"items-center",
											"justify-center",
										);
										const fallback = document.createElement("div");
										fallback.className =
											"flex flex-col items-center justify-center gap-2 text-muted-foreground";
										fallback.innerHTML = `
											<svg class="w-16 h-16" fill="none" stroke="currentColor" viewBox="0 0 24 24">
												<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
											</svg>
											<span>Failed to load image</span>
										`;
										e.currentTarget.parentElement?.appendChild(fallback);
									}}
								/>
							</div>
							<div className="space-y-2">
								<div>
									<p className="text-sm font-medium text-foreground">Path</p>
									<p className="text-sm text-muted-foreground font-mono">
										{currentWallpaper.path}
									</p>
								</div>
								<div>
									<p className="text-sm font-medium text-foreground">Monitor</p>
									<p className="text-sm text-muted-foreground">
										{currentWallpaper.monitor || "All monitors"}
									</p>
								</div>
								<div>
									<p className="text-sm font-medium text-foreground">Fit Mode</p>
									<p className="text-sm text-muted-foreground">
										{currentWallpaper.fit_mode}
									</p>
								</div>
							</div>
						</div>
					) : (
						<div className="flex flex-col items-center justify-center h-[400px] text-muted-foreground">
							<ImageIcon className="w-16 h-16 mb-4" />
							<p className="text-lg font-medium text-foreground">No wallpaper configured</p>
							<p className="text-sm text-muted-foreground">Click "Change Wallpaper" to set one</p>
						</div>
					)}
				</CardContent>
			</Card>

			{matugenExists && (
				<Card>
					<CardHeader>
						<CardTitle>General</CardTitle>
						<CardDescription>
							Appearance settings and theme synchronization
						</CardDescription>
					</CardHeader>
					<CardContent className="space-y-6">
						<div>
							<h3 className="text-base font-semibold mb-4">Matugen</h3>
							<div className="grid grid-cols-2 gap-6">
								<div className="flex items-center justify-between">
									<div className="space-y-0.5">
										<Label htmlFor="sync-matugen" className="text-sm font-medium">
											Enable
										</Label>
										<p className="text-sm text-muted-foreground">
											Automatically generate color scheme from wallpaper
										</p>
									</div>
									<Switch
										id="sync-matugen"
										checked={syncMatugen}
										onCheckedChange={handleSyncMatugenChange}
									/>
								</div>

								<div className="flex items-center justify-between">
									<div className="space-y-0.5">
										<Label
											htmlFor="matugen-light-mode"
											className={`text-sm font-medium ${!syncMatugen ? 'opacity-50' : ''}`}
										>
											Light Mode
										</Label>
										<p className={`text-sm text-muted-foreground ${!syncMatugen ? 'opacity-50' : ''}`}>
											Use light color scheme instead of dark
										</p>
									</div>
									<Switch
										id="matugen-light-mode"
										checked={matugenLightMode}
										onCheckedChange={handleLightModeChange}
										disabled={!syncMatugen}
									/>
								</div>
							</div>

							<div className="space-y-2">
								<Label htmlFor="generator-type" className="text-sm font-medium">
									Generator Type
								</Label>
								<Select
									value={generatorType}
									onValueChange={handleGeneratorTypeChange}
									disabled={!syncMatugen}
								>
									<SelectTrigger
										id="generator-type"
										className={!syncMatugen ? "opacity-50" : ""}
									>
										<SelectValue placeholder="Select generator type" />
									</SelectTrigger>
									<SelectContent>
										<SelectItem value="scheme-content">Scheme Content</SelectItem>
										<SelectItem value="scheme-expressive">Scheme Expressive</SelectItem>
										<SelectItem value="scheme-fidelity">Scheme Fidelity</SelectItem>
										<SelectItem value="scheme-fruit-salad">Scheme Fruit Salad</SelectItem>
										<SelectItem value="scheme-monochrome">Scheme Monochrome</SelectItem>
										<SelectItem value="scheme-neutral">Scheme Neutral</SelectItem>
										<SelectItem value="scheme-rainbow">Scheme Rainbow</SelectItem>
										<SelectItem value="scheme-tonal-spot">Scheme Tonal Spot</SelectItem>
										<SelectItem value="scheme-vibrant">Scheme Vibrant</SelectItem>
									</SelectContent>
								</Select>
								<p className={`text-sm text-muted-foreground ${!syncMatugen ? 'opacity-50' : ''}`}>
									Color scheme generation algorithm
								</p>
							</div>
						</div>

						<div>
							<h3 className="text-base font-semibold mb-4">Theme</h3>
							<div className="flex gap-3">
								<Button
									variant="outline"
									onClick={async () => {
										try {
											const templatePath = await ensureMatugenTemplate();
											toast.success(`Template created at ${templatePath}`);
										} catch (err) {
											toast.error(`Failed to create template: ${err}`);
										}
									}}
								>
									<FileCode className="h-4 w-4 mr-2" />
									Setup Template
								</Button>
								<Button
									variant="outline"
									onClick={async () => {
										try {
											// Run matugen on current wallpaper if available
											if (currentWallpaper?.path) {
												await invoke("run_matugen", {
													imagePath: currentWallpaper.path,
													lightMode: matugenLightMode,
													generatorType: generatorType,
												});
											}
											// Reload the theme CSS
											await reloadTheme();
											toast.success("Theme regenerated and reloaded");
										} catch (err) {
											toast.error(`Failed to reload theme: ${err}`);
										}
									}}
									disabled={!currentWallpaper}
								>
									<Palette className="h-4 w-4 mr-2" />
									Reload Theme
								</Button>
							</div>
							<p className="text-sm text-muted-foreground mt-2">
								Setup Template creates the matugen template at ~/.config/matugen/templates/hyprconfig.css.
								Reload Theme runs matugen on the current wallpaper and reloads the CSS.
							</p>
						</div>
					</CardContent>
				</Card>
			)}

		</div>
	);
}
