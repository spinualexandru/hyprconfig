import { Pencil, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
import { TableCell, TableRow } from "@/components/ui/table";
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

interface KeybindTableRowProps {
  keybind: Keybind;
  index: number;
  variables: Variable[];
  onEdit: (keybind: Keybind, index: number) => void;
  onDelete: (index: number) => void;
}

export function KeybindTableRow({
  keybind,
  index,
  variables,
  onEdit,
  onDelete,
}: KeybindTableRowProps) {
  return (
    <TableRow>
      <TableCell>
        <ModifierBadges modifiers={keybind.modifiers} />
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
        <CommandWithVariables command={keybind.params} variables={variables} />
      </TableCell>
      <TableCell>
        <div className="flex gap-1">
          <Button
            size="icon"
            variant="ghost"
            onClick={() => onEdit(keybind, index)}
          >
            <Pencil className="h-4 w-4" />
          </Button>
          <Button
            size="icon"
            variant="ghost"
            onClick={() => onDelete(index)}
          >
            <Trash2 className="h-4 w-4 text-destructive" />
          </Button>
        </div>
      </TableCell>
    </TableRow>
  );
}
