import { invoke } from "@tauri-apps/api/core";
import { DollarSign, RefreshCw } from "lucide-react";
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
import { Skeleton } from "@/components/ui/skeleton";
import type { Variable } from "@/types/variables";

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

	// Use cached variables if loading and cache exists, otherwise use current variables
	const displayVariables =
		loading && cachedVariables.length > 0 ? cachedVariables : variables;

	return (
		<div className="p-6 space-y-6">
			<div className="flex items-center justify-between">
				<div>
					<h1 className="text-3xl font-bold tracking-tight">Variables</h1>
					<p className="text-muted-foreground mt-2">
						View and manage your Hyprland configuration variables
					</p>
				</div>
				<Button onClick={loadVariables} variant="outline" disabled={loading}>
					<RefreshCw className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`} />
					Refresh
				</Button>
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
							<Table>
								<TableHeader>
									<TableRow>
										<TableHead className="w-[250px]">Name</TableHead>
										<TableHead>Value</TableHead>
									</TableRow>
								</TableHeader>
								<TableBody>
									{displayVariables.map((variable, index) => (
										<TableRow key={index}>
											<TableCell>
												<div className="flex items-center gap-2">
													<Badge variant="secondary">
														<code className="text-xs">${variable.name}</code>
													</Badge>
												</div>
											</TableCell>
											<TableCell>
												<code className="relative rounded bg-muted px-[0.3rem] py-[0.2rem] font-mono text-sm">
													{variable.value}
												</code>
											</TableCell>
										</TableRow>
									))}
								</TableBody>
							</Table>
						</div>
					</CardContent>
				</Card>
			)}
		</div>
	);
}
