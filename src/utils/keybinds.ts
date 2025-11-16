import type { Variable } from "@/types/variables";
import { DISPATCHER_DESCRIPTIONS } from "@/constants/keybinds";

// Modifier color mapping
export const getModifierVariant = (
  modifier: string,
): "default" | "secondary" | "outline" | "destructive" => {
  const mod = modifier.toUpperCase();
  if (mod === "SUPER" || mod === "MOD4" || mod === "WIN") return "default";
  if (mod === "SHIFT") return "secondary";
  if (mod === "ALT" || mod === "MOD1") return "outline";
  if (mod === "CTRL" || mod === "CONTROL") return "destructive";
  return "outline";
};

// Get dispatcher description from mapping
export const getDispatcherDescription = (dispatcher: string): string => {
  return (
    DISPATCHER_DESCRIPTIONS[dispatcher.toLowerCase()] ||
    "Custom dispatcher command"
  );
};

// Helper function to parse command and extract variables
export const parseCommandVariables = (
  command: string,
): { text: string; isVariable: boolean }[] => {
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
export const getVariableValue = (
  varName: string,
  variables: Variable[],
): string | null => {
  const cleanName = varName.startsWith("$") ? varName.substring(1) : varName;
  const variable = variables.find((v) => v.name === cleanName);
  return variable ? variable.value : null;
};
