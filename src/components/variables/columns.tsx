import type { ColumnDef } from "@tanstack/react-table";
import { Trash2, Check, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import type { Variable } from "@/types/variables";

interface EditingState {
	editingIndex: number | null;
	editValue: string;
	editLoading: boolean;
}

interface CreateVariableColumnsOptions {
	editing: EditingState;
	onStartEdit: (index: number, value: string) => void;
	onSaveEdit: (name: string) => void;
	onCancelEdit: () => void;
	onEditValueChange: (value: string) => void;
	onDelete: (name: string) => void;
}

export function createVariableColumns({
	editing,
	onStartEdit,
	onSaveEdit,
	onCancelEdit,
	onEditValueChange,
	onDelete,
}: CreateVariableColumnsOptions): ColumnDef<Variable>[] {
	return [
		{
			accessorKey: "name",
			header: "Name",
			size: 250,
			cell: ({ row }) => (
				<div className="flex items-center gap-2">
					<Badge variant="secondary">
						<code className="text-xs">${row.original.name}</code>
					</Badge>
				</div>
			),
		},
		{
			accessorKey: "value",
			header: "Value",
			cell: ({ row }) => {
				const isEditing = editing.editingIndex === row.index;

				if (isEditing) {
					return (
						<div className="flex items-center gap-2">
							<Input
								value={editing.editValue}
								onChange={(e) => onEditValueChange(e.target.value)}
								className="font-mono text-sm"
								disabled={editing.editLoading}
								onKeyDown={(e) => {
									if (e.key === "Enter") {
										onSaveEdit(row.original.name);
									} else if (e.key === "Escape") {
										onCancelEdit();
									}
								}}
								autoFocus
							/>
							<Button
								size="icon"
								variant="ghost"
								onClick={() => onSaveEdit(row.original.name)}
								disabled={editing.editLoading}
							>
								<Check className="h-4 w-4" />
							</Button>
							<Button
								size="icon"
								variant="ghost"
								onClick={onCancelEdit}
								disabled={editing.editLoading}
							>
								<X className="h-4 w-4" />
							</Button>
						</div>
					);
				}

				return (
					<code
						className="relative rounded bg-muted px-[0.3rem] py-[0.2rem] font-mono text-sm cursor-pointer hover:bg-muted/80"
						onClick={() => onStartEdit(row.index, row.original.value)}
					>
						{row.original.value}
					</code>
				);
			},
		},
		{
			id: "actions",
			header: "Actions",
			size: 100,
			cell: ({ row }) => {
				const isEditing = editing.editingIndex === row.index;

				if (isEditing) {
					return null;
				}

				return (
					<Button
						size="icon"
						variant="ghost"
						onClick={() => onDelete(row.original.name)}
					>
						<Trash2 className="h-4 w-4 text-destructive" />
					</Button>
				);
			},
		},
	];
}
