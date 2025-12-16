import type { ColumnDef } from "@tanstack/react-table";
import { Pencil, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import type { Keybind } from "@/types/keybinds";
import type { Variable } from "@/types/variables";
import { getDispatcherDescription } from "@/utils/keybinds";
import { CommandWithVariables } from "./CommandWithVariables";
import { ModifierBadges } from "./ModifierBadges";

interface CreateKeybindColumnsOptions {
	variables: Variable[];
	onEdit: (keybind: Keybind, index: number) => void;
	onDelete: (index: number) => void;
}

export function createKeybindColumns({
	variables,
	onEdit,
	onDelete,
}: CreateKeybindColumnsOptions): ColumnDef<Keybind>[] {
	return [
		{
			accessorKey: "modifiers",
			header: "Modifiers",
			size: 200,
			cell: ({ row }) => <ModifierBadges modifiers={row.original.modifiers} />,
		},
		{
			accessorKey: "key",
			header: "Key",
			size: 150,
			cell: ({ row }) => <Kbd>{row.original.key}</Kbd>,
		},
		{
			accessorKey: "dispatcher",
			header: "Action",
			size: 150,
			cell: ({ row }) => (
				<Tooltip>
					<TooltipTrigger asChild>
						<code className="relative rounded bg-muted px-[0.3rem] py-[0.2rem] font-mono text-sm cursor-help">
							{row.original.dispatcher}
						</code>
					</TooltipTrigger>
					<TooltipContent>
						<p>{getDispatcherDescription(row.original.dispatcher)}</p>
					</TooltipContent>
				</Tooltip>
			),
		},
		{
			accessorKey: "params",
			header: "Command",
			cell: ({ row }) => (
				<CommandWithVariables command={row.original.params} variables={variables} />
			),
		},
		{
			id: "actions",
			header: "Actions",
			size: 100,
			cell: ({ row }) => (
				<div className="flex gap-1">
					<Button
						size="icon"
						variant="ghost"
						onClick={() => onEdit(row.original, row.index)}
					>
						<Pencil className="h-4 w-4" />
					</Button>
					<Button
						size="icon"
						variant="ghost"
						onClick={() => onDelete(row.index)}
					>
						<Trash2 className="h-4 w-4 text-destructive" />
					</Button>
				</div>
			),
		},
	];
}
