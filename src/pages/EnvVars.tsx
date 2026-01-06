import { invoke } from "@tauri-apps/api/core";
import { Terminal, RefreshCw, Plus } from "lucide-react";
import { useEffect, useState, useMemo } from "react";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { DataTable } from "@/components/ui/data-table";
import { Skeleton } from "@/components/ui/skeleton";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { EnvVar } from "@/types/envvars";
import { createEnvVarColumns } from "@/components/envvars";

function EnvVarsTableSkeleton() {
	return (
		<Card>
			<CardHeader>
				<div className="flex items-center gap-3">
					<div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
						<Terminal className="h-5 w-5 text-primary" />
					</div>
					<div>
						<CardTitle>Environment Variables</CardTitle>
						<CardDescription>
							Your Hyprland environment variables
						</CardDescription>
					</div>
				</div>
			</CardHeader>
			<CardContent>
				<div className="space-y-4">
					{[...Array(6)].map((_, i) => (
						<div key={i} className="flex items-center gap-4">
							<Skeleton className="h-6 w-32" />
							<Skeleton className="h-6 flex-1" />
						</div>
					))}
				</div>
			</CardContent>
		</Card>
	);
}

export default function EnvVars() {
	const [envVars, setEnvVars] = useState<EnvVar[]>([]);
	const [cachedEnvVars, setCachedEnvVars] = useState<EnvVar[]>([]);
	const [loading, setLoading] = useState(false);
	const [initialLoad, setInitialLoad] = useState(true);
	const [error, setError] = useState<string | null>(null);

	// Add EnvVar Dialog State
	const [showAddDialog, setShowAddDialog] = useState(false);
	const [newVarName, setNewVarName] = useState("");
	const [newVarValue, setNewVarValue] = useState("");
	const [addError, setAddError] = useState<string | null>(null);
	const [addLoading, setAddLoading] = useState(false);

	// Inline Editing State
	const [editingRowIndex, setEditingRowIndex] = useState<number | null>(null);
	const [editValue, setEditValue] = useState("");
	const [editLoading, setEditLoading] = useState(false);

	useEffect(() => {
		loadEnvVars();
	}, []);

	const loadEnvVars = async () => {
		setLoading(true);
		setError(null);

		invoke<EnvVar[]>("get_env_vars")
			.then((result) => {
				setEnvVars(result);
				setCachedEnvVars(result);
			})
			.catch((err) => {
				setError(err as string);
				if (cachedEnvVars.length === 0) {
					setEnvVars([]);
				}
			})
			.finally(() => {
				setLoading(false);
				setInitialLoad(false);
			});
	};

	const handleAddEnvVar = async () => {
		if (!newVarName.trim()) {
			setAddError("Environment variable name is required");
			return;
		}

		setAddLoading(true);
		setAddError(null);

		invoke("add_env_var", { name: newVarName.trim(), value: newVarValue })
			.then(() => {
				setShowAddDialog(false);
				setNewVarName("");
				setNewVarValue("");
				loadEnvVars();
			})
			.catch((err) => {
				setAddError(err as string);
			})
			.finally(() => {
				setAddLoading(false);
			});
	};

	const handleStartEdit = (rowIndex: number, currentValue: string) => {
		setEditingRowIndex(rowIndex);
		setEditValue(currentValue);
	};

	const handleCancelEdit = () => {
		setEditingRowIndex(null);
		setEditValue("");
	};

	const handleSaveEdit = async (envIndex: number, name: string) => {
		setEditLoading(true);

		invoke("edit_env_var", { index: envIndex, name, value: editValue })
			.then(() => {
				setEditingRowIndex(null);
				setEditValue("");
				loadEnvVars();
			})
			.catch((err) => {
				setError(err as string);
			})
			.finally(() => {
				setEditLoading(false);
			});
	};

	const handleDeleteEnvVar = async (envIndex: number) => {
		invoke("delete_env_var", { index: envIndex })
			.then(() => {
				loadEnvVars();
			})
			.catch((err) => {
				setError(err as string);
			});
	};

	const displayEnvVars =
		loading && cachedEnvVars.length > 0 ? cachedEnvVars : envVars;

	const columns = useMemo(
		() =>
			createEnvVarColumns({
				editing: {
					editingRowIndex,
					editValue,
					editLoading,
				},
				onStartEdit: handleStartEdit,
				onSaveEdit: handleSaveEdit,
				onCancelEdit: handleCancelEdit,
				onEditValueChange: setEditValue,
				onDelete: handleDeleteEnvVar,
			}),
		[editingRowIndex, editValue, editLoading],
	);

	return (
		<div className="p-6 space-y-6">
			<div className="flex items-center justify-between">
				<div>
					<h1 className="text-3xl font-bold tracking-tight text-foreground">
						Environment Variables
					</h1>
					<p className="text-muted-foreground mt-2">
						View and manage your Hyprland environment variables
					</p>
				</div>
				<div className="flex gap-2">
					<Button onClick={() => setShowAddDialog(true)} variant="default">
						<Plus className="h-4 w-4 mr-2" />
						Add Env Var
					</Button>
					<Button onClick={loadEnvVars} variant="outline" disabled={loading}>
						<RefreshCw
							className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`}
						/>
						Refresh
					</Button>
				</div>
			</div>

			{/* Error Display */}
			{error && cachedEnvVars.length === 0 && (
				<Card className="border-destructive">
					<CardHeader>
						<CardTitle className="text-destructive">
							Error Loading Environment Variables
						</CardTitle>
					</CardHeader>
					<CardContent>
						<p className="text-sm text-muted-foreground">{error}</p>
					</CardContent>
				</Card>
			)}

			{/* EnvVars Display */}
			{(loading && cachedEnvVars.length === 0) || initialLoad ? (
				<EnvVarsTableSkeleton />
			) : displayEnvVars.length === 0 ? (
				<Card>
					<CardContent className="pt-6">
						<p className="text-sm text-muted-foreground">
							No environment variables found in your Hyprland configuration.
						</p>
					</CardContent>
				</Card>
			) : (
				<Card>
					<CardHeader>
						<div className="flex items-center gap-3">
							<div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
								<Terminal className="h-5 w-5 text-primary" />
							</div>
							<div>
								<CardTitle>
									Environment Variables ({displayEnvVars.length})
								</CardTitle>
								<CardDescription>
									Your Hyprland environment variables
								</CardDescription>
							</div>
						</div>
					</CardHeader>
					<CardContent>
						<div className="rounded-md border">
							<DataTable columns={columns} data={displayEnvVars} />
						</div>
					</CardContent>
				</Card>
			)}

			{/* Add EnvVar Dialog */}
			<Dialog open={showAddDialog} onOpenChange={setShowAddDialog}>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>Add New Environment Variable</DialogTitle>
						<DialogDescription>
							Create a new environment variable in your Hyprland configuration.
						</DialogDescription>
					</DialogHeader>
					<div className="space-y-4 py-4">
						<div className="space-y-2">
							<Label htmlFor="env-name">Variable Name</Label>
							<Input
								id="env-name"
								placeholder="e.g., XCURSOR_SIZE, QT_QPA_PLATFORM"
								value={newVarName}
								onChange={(e) => {
									setNewVarName(e.target.value);
									setAddError(null);
								}}
								disabled={addLoading}
							/>
							<p className="text-xs text-muted-foreground">
								Letters, numbers, and underscores only. Convention: UPPER_CASE.
							</p>
						</div>
						<div className="space-y-2">
							<Label htmlFor="env-value">Value</Label>
							<Input
								id="env-value"
								placeholder="e.g., 24, wayland, qt6ct"
								value={newVarValue}
								onChange={(e) => setNewVarValue(e.target.value)}
								disabled={addLoading}
								onKeyDown={(e) => {
									if (e.key === "Enter") {
										handleAddEnvVar();
									}
								}}
							/>
						</div>
						{addError && <p className="text-sm text-destructive">{addError}</p>}
					</div>
					<DialogFooter>
						<Button
							variant="outline"
							onClick={() => {
								setShowAddDialog(false);
								setNewVarName("");
								setNewVarValue("");
								setAddError(null);
							}}
							disabled={addLoading}
						>
							Cancel
						</Button>
						<Button onClick={handleAddEnvVar} disabled={addLoading}>
							{addLoading ? "Adding..." : "Add Environment Variable"}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	);
}
