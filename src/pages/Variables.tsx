import { invoke } from "@tauri-apps/api/core";
import { DollarSign, RefreshCw, Plus } from "lucide-react";
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
import type { Variable } from "@/types/variables";
import { createVariableColumns } from "@/components/variables";

function VariablesTableSkeleton() {
	return (
		<Card>
			<CardHeader>
				<div className="flex items-center gap-3">
					<div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
						<DollarSign className="h-5 w-5 text-primary" />
					</div>
					<div>
						<CardTitle>Variables</CardTitle>
						<CardDescription>
							Your Hyprland configuration variables
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

export default function Variables() {
	const [variables, setVariables] = useState<Variable[]>([]);
	const [cachedVariables, setCachedVariables] = useState<Variable[]>([]);
	const [loading, setLoading] = useState(false);
	const [initialLoad, setInitialLoad] = useState(true);
	const [error, setError] = useState<string | null>(null);

	// Add Variable Dialog State
	const [showAddDialog, setShowAddDialog] = useState(false);
	const [newVarName, setNewVarName] = useState("");
	const [newVarValue, setNewVarValue] = useState("");
	const [addError, setAddError] = useState<string | null>(null);
	const [addLoading, setAddLoading] = useState(false);

	// Inline Editing State
	const [editingIndex, setEditingIndex] = useState<number | null>(null);
	const [editValue, setEditValue] = useState("");
	const [editLoading, setEditLoading] = useState(false);

	useEffect(() => {
		loadVariables();
	}, []);

	const loadVariables = async () => {
		setLoading(true);
		setError(null);

		invoke<Variable[]>("get_variables")
			.then((result) => {
				setVariables(result);
				setCachedVariables(result); // Cache for next time
			})
			.catch((err) => {
				setError(err as string);
				// Keep showing cached data if available
				if (cachedVariables.length === 0) {
					setVariables([]);
				}
			})
			.finally(() => {
				setLoading(false);
				setInitialLoad(false);
			});
	};

	const handleAddVariable = async () => {
		if (!newVarName.trim()) {
			setAddError("Variable name is required");
			return;
		}

		setAddLoading(true);
		setAddError(null);

		invoke("add_variable", { name: newVarName.trim(), value: newVarValue })
			.then(() => {
				// Success - close dialog and reload
				setShowAddDialog(false);
				setNewVarName("");
				setNewVarValue("");
				loadVariables();
			})
			.catch((err) => {
				setAddError(err as string);
			})
			.finally(() => {
				setAddLoading(false);
			});
	};

	const handleStartEdit = (index: number, currentValue: string) => {
		setEditingIndex(index);
		setEditValue(currentValue);
	};

	const handleCancelEdit = () => {
		setEditingIndex(null);
		setEditValue("");
	};

	const handleSaveEdit = async (varName: string) => {
		setEditLoading(true);

		invoke("set_variable", { name: varName, value: editValue })
			.then(() => {
				// Success - exit edit mode and reload
				setEditingIndex(null);
				setEditValue("");
				loadVariables();
			})
			.catch((err) => {
				setError(err as string);
			})
			.finally(() => {
				setEditLoading(false);
			});
	};

	const handleDeleteVariable = async (varName: string) => {
		invoke("delete_variable", { name: varName })
			.then(() => {
				// Success - reload variables
				loadVariables();
			})
			.catch((err) => {
				setError(err as string);
			});
	};

	// Use cached variables if loading and cache exists, otherwise use current variables
	const displayVariables =
		loading && cachedVariables.length > 0 ? cachedVariables : variables;

	const columns = useMemo(
		() =>
			createVariableColumns({
				editing: {
					editingIndex,
					editValue,
					editLoading,
				},
				onStartEdit: handleStartEdit,
				onSaveEdit: handleSaveEdit,
				onCancelEdit: handleCancelEdit,
				onEditValueChange: setEditValue,
				onDelete: handleDeleteVariable,
			}),
		[editingIndex, editValue, editLoading],
	);

	return (
		<div className="p-6 space-y-6">
			<div className="flex items-center justify-between">
				<div>
					<h1 className="text-3xl font-bold tracking-tight text-foreground">Variables</h1>
					<p className="text-muted-foreground mt-2">
						View and manage your Hyprland configuration variables
					</p>
				</div>
				<div className="flex gap-2">
					<Button
						onClick={() => setShowAddDialog(true)}
						variant="default"
					>
						<Plus className="h-4 w-4 mr-2" />
						Add Variable
					</Button>
					<Button onClick={loadVariables} variant="outline" disabled={loading}>
						<RefreshCw className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`} />
						Refresh
					</Button>
				</div>
			</div>

			{/* Error Display */}
			{error && cachedVariables.length === 0 && (
				<Card className="border-destructive">
					<CardHeader>
						<CardTitle className="text-destructive">
							Error Loading Variables
						</CardTitle>
					</CardHeader>
					<CardContent>
						<p className="text-sm text-muted-foreground">{error}</p>
					</CardContent>
				</Card>
			)}

			{/* Variables Display */}
			{(loading && cachedVariables.length === 0) || initialLoad ? (
				<VariablesTableSkeleton />
			) : displayVariables.length === 0 ? (
				<Card>
					<CardContent className="pt-6">
						<p className="text-sm text-muted-foreground">
							No variables found in your Hyprland configuration.
						</p>
					</CardContent>
				</Card>
			) : (
				<Card>
					<CardHeader>
						<div className="flex items-center gap-3">
							<div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
								<DollarSign className="h-5 w-5 text-primary" />
							</div>
							<div>
								<CardTitle>Variables ({displayVariables.length})</CardTitle>
								<CardDescription>
									Your Hyprland configuration variables
								</CardDescription>
							</div>
						</div>
					</CardHeader>
					<CardContent>
						<div className="rounded-md border">
							<DataTable columns={columns} data={displayVariables} />
						</div>
					</CardContent>
				</Card>
			)}

			{/* Add Variable Dialog */}
			<Dialog open={showAddDialog} onOpenChange={setShowAddDialog}>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>Add New Variable</DialogTitle>
						<DialogDescription>
							Create a new variable in your Hyprland configuration.
						</DialogDescription>
					</DialogHeader>
					<div className="space-y-4 py-4">
						<div className="space-y-2">
							<Label htmlFor="var-name">Variable Name</Label>
							<Input
								id="var-name"
								placeholder="e.g., terminal, gaps, mainMod"
								value={newVarName}
								onChange={(e) => {
									setNewVarName(e.target.value);
									setAddError(null);
								}}
								disabled={addLoading}
							/>
							<p className="text-xs text-muted-foreground">
								Letters, numbers, and underscores only. No spaces.
							</p>
						</div>
						<div className="space-y-2">
							<Label htmlFor="var-value">Value</Label>
							<Input
								id="var-value"
								placeholder="e.g., kitty, 5, SUPER"
								value={newVarValue}
								onChange={(e) => setNewVarValue(e.target.value)}
								disabled={addLoading}
								onKeyDown={(e) => {
									if (e.key === "Enter") {
										handleAddVariable();
									}
								}}
							/>
						</div>
						{addError && (
							<p className="text-sm text-destructive">{addError}</p>
						)}
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
						<Button onClick={handleAddVariable} disabled={addLoading}>
							{addLoading ? "Adding..." : "Add Variable"}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	);
}
