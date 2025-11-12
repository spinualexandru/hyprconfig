import { invoke } from "@tauri-apps/api/core";
import { Keyboard, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Kbd } from "@/components/ui/kbd";
import { Skeleton } from "@/components/ui/skeleton";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import type { Keybind } from "@/types/keybinds";
import type { Variable } from "@/types/variables";

// Modifier color mapping
const getModifierVariant = (modifier: string): "default" | "secondary" | "outline" | "destructive" => {
	const mod = modifier.toUpperCase();
	if (mod === "SUPER" || mod === "MOD4" || mod === "WIN") return "default";
	if (mod === "SHIFT") return "secondary";
	if (mod === "ALT" || mod === "MOD1") return "outline";
	if (mod === "CTRL" || mod === "CONTROL") return "destructive";
	return "outline";
};

// Dispatcher descriptions mapping
const getDispatcherDescription = (dispatcher: string): string => {
	const descriptions: Record<string, string> = {
		exec: "Execute a shell command",
		killactive: "Close the active window",
		togglefloating: "Toggle floating mode for the active window",
		fullscreen: "Toggle fullscreen mode",
		workspace: "Switch to a specific workspace",
		movetoworkspace: "Move active window to a workspace",
		movetoworkspacesilent: "Move window to workspace without switching to it",
		togglegroup: "Toggle the current window's group",
		changegroupactive: "Change the active window in a group",
		togglesplit: "Toggle split direction",
		swapsplit: "Swap split direction",
		splitratio: "Change split ratio",
		movefocus: "Move focus in a direction (l/r/u/d)",
		movewindow: "Move window in a direction",
		resizeactive: "Resize the active window",
		centerwindow: "Center the active window",
		cyclenext: "Cycle to the next window",
		swapnext: "Swap with the next window",
		focusmonitor: "Focus a specific monitor",
		movecursortocorner: "Move cursor to a corner",
		movecursor: "Move cursor to coordinates",
		exit: "Exit Hyprland",
		forcerendererreload: "Force renderer reload",
		movecurrentworkspacetomonitor: "Move current workspace to a monitor",
		focusworkspaceoncurrentmonitor: "Focus workspace on current monitor",
		focusurgentorlast: "Focus urgent or last window",
		togglespecialworkspace: "Toggle special workspace (scratchpad)",
		movetoworkspace: "Move window to workspace",
		pin: "Pin the active window",
		mouse: "Mouse button action",
		bringactivetotop: "Bring active window to top",
		alterzorder: "Change window z-order (top/bottom)",
		lockgroups: "Lock/unlock window groups",
		lockactivegroup: "Lock/unlock the active group",
		moveintogroup: "Move window into a group",
		moveoutofgroup: "Move window out of group",
		movegroupwindow: "Move window within group",
		global: "Global shortcut (works when not focused)",
		pass: "Pass key to active window",
		sendshortcut: "Send shortcut to a window",
		dpms: "Control monitor power (on/off)",
		pseudo: "Toggle pseudo-tiling mode",
		layoutmsg: "Send message to layout",
		toggleopaque: "Toggle window opacity",
		submap: "Switch to a key submap",
	};

	return descriptions[dispatcher.toLowerCase()] || "Custom dispatcher command";
};

// Helper function to parse command and extract variables
const parseCommandVariables = (command: string): { text: string; isVariable: boolean }[] => {
	const parts: { text: string; isVariable: boolean }[] = [];
	let currentText = "";
	let i = 0;

	while (i < command.length) {
		if (command[i] === "$") {
			// Save any accumulated text
			if (currentText) {
				parts.push({ text: currentText, isVariable: false });
				currentText = "";
			}

			// Extract variable name (alphanumeric and underscore)
			let varName = "$";
			i++;
			while (i < command.length && /[a-zA-Z0-9_]/.test(command[i])) {
				varName += command[i];
				i++;
			}

			if (varName.length > 1) {
				parts.push({ text: varName, isVariable: true });
			} else {
				currentText += "$";
			}
		} else {
			currentText += command[i];
			i++;
		}
	}

	// Add any remaining text
	if (currentText) {
		parts.push({ text: currentText, isVariable: false });
	}

	return parts;
};

// Helper function to get variable value by name
const getVariableValue = (varName: string, variables: Variable[]): string | null => {
	const cleanName = varName.startsWith("$") ? varName.substring(1) : varName;
	const variable = variables.find((v) => v.name === cleanName);
	return variable ? variable.value : null;
};

// Component to render command with variable tooltips
function CommandWithVariables({ command, variables }: { command: string; variables: Variable[] }) {
	const parts = parseCommandVariables(command);

	if (parts.length === 0 || !parts.some((p) => p.isVariable)) {
		// No variables, render plain text
		return <span className="text-sm text-muted-foreground">{command || "—"}</span>;
	}

	return (
		<span className="text-sm text-muted-foreground">
			{parts.map((part, index) => {
				if (part.isVariable) {
					const value = getVariableValue(part.text, variables);
					if (value) {
						return (
							<Tooltip key={index}>
								<TooltipTrigger asChild>
									<span className="text-primary cursor-help font-medium">
										{part.text}
									</span>
								</TooltipTrigger>
								<TooltipContent>
									<p className="font-mono text-xs">{value}</p>
								</TooltipContent>
							</Tooltip>
						);
					}
					return <span key={index}>{part.text}</span>;
				}
				return <span key={index}>{part.text}</span>;
			})}
		</span>
	);
}

function KeybindsTableSkeleton() {
	return (
		<Card>
			<CardHeader>
				<div className="flex items-center gap-3">
					<div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
						<Keyboard className="h-5 w-5 text-primary" />
					</div>
					<div>
						<CardTitle>Keybinds</CardTitle>
						<CardDescription>
							Your Hyprland keyboard shortcuts
						</CardDescription>
					</div>
				</div>
			</CardHeader>
			<CardContent>
				<div className="space-y-4">
					{[...Array(8)].map((_, i) => (
						<div key={i} className="flex items-center gap-4">
							<Skeleton className="h-6 w-32" />
							<Skeleton className="h-6 w-16" />
							<Skeleton className="h-6 w-24" />
							<Skeleton className="h-6 flex-1" />
						</div>
					))}
				</div>
			</CardContent>
		</Card>
	);
}

export default function Keybinds() {
	const [keybinds, setKeybinds] = useState<Keybind[]>([]);
	const [cachedKeybinds, setCachedKeybinds] = useState<Keybind[]>([]);
	const [variables, setVariables] = useState<Variable[]>([]);
	const [loading, setLoading] = useState(false);
	const [initialLoad, setInitialLoad] = useState(true);
	const [error, setError] = useState<string | null>(null);

	useEffect(() => {
		loadKeybinds();
		loadVariables();
	}, []);

	const loadVariables = async () => {
		invoke<Variable[]>("get_variables")
			.then((result) => {
				setVariables(result);
			})
			.catch((err) => {
				// Silently fail - variables are optional enhancement
				console.error("Failed to load variables:", err);
			});
	};

	const loadKeybinds = async () => {
		setLoading(true);
		setError(null);

		invoke<Keybind[]>("get_keybinds")
			.then((result) => {
				setKeybinds(result);
				setCachedKeybinds(result); // Cache for next time
			})
			.catch((err) => {
				setError(err as string);
				// Keep showing cached data if available
				if (cachedKeybinds.length === 0) {
					setKeybinds([]);
				}
			})
			.finally(() => {
				setLoading(false);
				setInitialLoad(false);
			});
	};

	// Use cached keybinds if loading and cache exists, otherwise use current keybinds
	const displayKeybinds =
		loading && cachedKeybinds.length > 0 ? cachedKeybinds : keybinds;

	return (
		<div className="p-6 space-y-6">
			<div className="flex items-center justify-between">
				<div>
					<h1 className="text-3xl font-bold tracking-tight">Keybinds</h1>
					<p className="text-muted-foreground mt-2">
						View and change your Hyprland keyboard shortcuts and keybindings
					</p>
				</div>
				<Button onClick={loadKeybinds} variant="outline" disabled={loading}>
					<RefreshCw className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`} />
					Refresh
				</Button>
			</div>

			{/* Error Display */}
			{error && cachedKeybinds.length === 0 && (
				<Card className="border-destructive">
					<CardHeader>
						<CardTitle className="text-destructive">
							Error Loading Keybinds
						</CardTitle>
					</CardHeader>
					<CardContent>
						<p className="text-sm text-muted-foreground">{error}</p>
					</CardContent>
				</Card>
			)}

			{/* Keybinds Display */}
			{(loading && cachedKeybinds.length === 0) || initialLoad ? (
				<KeybindsTableSkeleton />
			) : displayKeybinds.length === 0 ? (
				<Card>
					<CardContent className="pt-6">
						<p className="text-sm text-muted-foreground">
							No keybinds found in your Hyprland configuration.
						</p>
					</CardContent>
				</Card>
			) : (
				<Card>
					<CardHeader>
						<div className="flex items-center gap-3">
							<div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
								<Keyboard className="h-5 w-5 text-primary" />
							</div>
							<div>
								<CardTitle>Keybinds ({displayKeybinds.length})</CardTitle>
								<CardDescription>
									Your Hyprland keyboard shortcuts
								</CardDescription>
							</div>
						</div>
					</CardHeader>
					<CardContent>
						<div className="rounded-md border">
							<TooltipProvider>
								<Table>
									<TableHeader>
										<TableRow>
											<TableHead className="w-[200px]">Modifiers</TableHead>
											<TableHead className="w-[150px]">Key</TableHead>
											<TableHead className="w-[150px]">Action</TableHead>
											<TableHead>Command</TableHead>
										</TableRow>
									</TableHeader>
									<TableBody>
										{displayKeybinds.map((keybind, index) => (
											<TableRow key={index}>
												<TableCell>
													<div className="flex gap-1 flex-wrap">
														{keybind.modifiers.length > 0 ? (
															keybind.modifiers.map((mod, i) => (
																<Badge
																	key={i}
																	variant={getModifierVariant(mod)}
																>
																	{mod}
																</Badge>
															))
														) : (
															<span className="text-muted-foreground text-sm">
																None
															</span>
														)}
													</div>
												</TableCell>
												<TableCell>
													<Kbd>{keybind.key}</Kbd>
												</TableCell>
												<TableCell>
													<Tooltip>
														<TooltipTrigger asChild>
															<code className="relative rounded bg-muted px-[0.3rem] py-[0.2rem] font-mono text-sm cursor-help">
																{keybind.dispatcher}
															</code>
														</TooltipTrigger>
														<TooltipContent>
															<p>{getDispatcherDescription(keybind.dispatcher)}</p>
														</TooltipContent>
													</Tooltip>
												</TableCell>
												<TableCell>
													<CommandWithVariables
														command={keybind.params}
														variables={variables}
													/>
												</TableCell>
											</TableRow>
										))}
									</TableBody>
								</Table>
							</TooltipProvider>
						</div>
					</CardContent>
				</Card>
			)}
		</div>
	);
}
