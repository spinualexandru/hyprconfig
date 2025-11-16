import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { Variable } from "@/types/variables";
import { parseCommandVariables, getVariableValue } from "@/utils/keybinds";

interface CommandWithVariablesProps {
  command: string;
  variables: Variable[];
}

export function CommandWithVariables({
  command,
  variables,
}: CommandWithVariablesProps) {
  const parts = parseCommandVariables(command);

  if (parts.length === 0 || !parts.some((p) => p.isVariable)) {
    // No variables, render plain text
    return (
      <span className="text-sm text-muted-foreground">{command || "—"}</span>
    );
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
