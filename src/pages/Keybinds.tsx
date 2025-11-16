import { invoke } from "@tauri-apps/api/core";
import { Keyboard, RefreshCw, Plus, Trash2, Pencil } from "lucide-react";
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
import { Checkbox } from "@/components/ui/checkbox";
import type { Keybind } from "@/types/keybinds";
import type { Variable } from "@/types/variables";

// Modifier color mapping
const getModifierVariant = (
  modifier: string,
): "default" | "secondary" | "outline" | "destructive" => {
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

// All available dispatchers (extracted from getDispatcherDescription)
const AVAILABLE_DISPATCHERS = [
  "exec",
  "killactive",
  "togglefloating",
  "fullscreen",
  "workspace",
  "movetoworkspace",
  "movetoworkspacesilent",
  "togglegroup",
  "changegroupactive",
  "togglesplit",
  "swapsplit",
  "splitratio",
  "movefocus",
  "movewindow",
  "resizeactive",
  "centerwindow",
  "cyclenext",
  "swapnext",
  "focusmonitor",
  "movecursortocorner",
  "movecursor",
  "exit",
  "forcerendererreload",
  "movecurrentworkspacetomonitor",
  "focusworkspaceoncurrentmonitor",
  "focusurgentorlast",
  "togglespecialworkspace",
  "movetoworkspace",
  "pin",
  "mouse",
  "bringactivetotop",
  "alterzorder",
  "lockgroups",
  "lockactivegroup",
  "moveintogroup",
  "moveoutofgroup",
  "movegroupwindow",
  "global",
  "pass",
  "sendshortcut",
  "dpms",
  "pseudo",
  "layoutmsg",
  "toggleopaque",
  "submap",
];

// Helper function to parse command and extract variables
const parseCommandVariables = (
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
const getVariableValue = (
  varName: string,
  variables: Variable[],
): string | null => {
  const cleanName = varName.startsWith("$") ? varName.substring(1) : varName;
  const variable = variables.find((v) => v.name === cleanName);
  return variable ? variable.value : null;
};

// Component to render command with variable tooltips
function CommandWithVariables({
  command,
  variables,
}: {
  command: string;
  variables: Variable[];
}) {
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
            <CardDescription>Your Hyprland keyboard shortcuts</CardDescription>
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

  // Keybind Form Dialog State
  const [showDialog, setShowDialog] = useState(false);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [formModifiers, setFormModifiers] = useState<string[]>([]);
  const [formKey, setFormKey] = useState("");
  const [formDispatcher, setFormDispatcher] = useState("");
  const [formParams, setFormParams] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const [formLoading, setFormLoading] = useState(false);
  const [dispatcherSuggestions, setDispatcherSuggestions] = useState<string[]>(
    [],
  );

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

  const resetForm = () => {
    setFormModifiers([]);
    setFormKey("");
    setFormDispatcher("");
    setFormParams("");
    setFormError(null);
    setEditingIndex(null);
  };

  const handleOpenAddDialog = () => {
    resetForm();
    setShowDialog(true);
  };

  const handleOpenEditDialog = (keybind: Keybind, index: number) => {
    setFormModifiers(keybind.modifiers);
    setFormKey(keybind.key);
    setFormDispatcher(keybind.dispatcher);
    setFormParams(keybind.params);
    setEditingIndex(index);
    setFormError(null);
    setShowDialog(true);
  };

  const handleCloseDialog = () => {
    setShowDialog(false);
    resetForm();
  };

  const toggleModifier = (modifier: string) => {
    if (formModifiers.includes(modifier)) {
      setFormModifiers(formModifiers.filter((m) => m !== modifier));
    } else {
      setFormModifiers([...formModifiers, modifier]);
    }
  };

  const handleDispatcherChange = (value: string) => {
    setFormDispatcher(value);
    // Update suggestions based on input
    if (value.trim()) {
      const filtered = AVAILABLE_DISPATCHERS.filter((d) =>
        d.toLowerCase().includes(value.toLowerCase()),
      ).slice(0, 8);
      setDispatcherSuggestions(filtered);
    } else {
      setDispatcherSuggestions([]);
    }
  };

  const handleSubmit = async () => {
    // Validation
    if (!formKey.trim()) {
      setFormError("Key is required");
      return;
    }
    if (!formDispatcher.trim()) {
      setFormError("Dispatcher is required");
      return;
    }

    setFormLoading(true);
    setFormError(null);

    const command = editingIndex !== null ? "edit_keybind" : "add_keybind";
    const args =
      editingIndex !== null
        ? {
            index: editingIndex,
            modifiers: formModifiers,
            key: formKey.trim(),
            dispatcher: formDispatcher.trim(),
            params: formParams.trim(),
          }
        : {
            modifiers: formModifiers,
            key: formKey.trim(),
            dispatcher: formDispatcher.trim(),
            params: formParams.trim(),
          };

    invoke(command, args)
      .then(() => {
        handleCloseDialog();
        loadKeybinds();
      })
      .catch((err) => {
        setFormError(err as string);
      })
      .finally(() => {
        setFormLoading(false);
      });
  };

  const handleDeleteKeybind = async (index: number) => {
    invoke("delete_keybind", { index })
      .then(() => {
        loadKeybinds();
      })
      .catch((err) => {
        setError(err as string);
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
        <div className="flex gap-2">
          <Button onClick={handleOpenAddDialog} variant="default">
            <Plus className="h-4 w-4 mr-2" />
            Add Keybind
          </Button>
          <Button onClick={loadKeybinds} variant="outline" disabled={loading}>
            <RefreshCw
              className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`}
            />
            Refresh
          </Button>
        </div>
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
                      <TableHead className="w-[100px]">Actions</TableHead>
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
                              <p>
                                {getDispatcherDescription(keybind.dispatcher)}
                              </p>
                            </TooltipContent>
                          </Tooltip>
                        </TableCell>
                        <TableCell>
                          <CommandWithVariables
                            command={keybind.params}
                            variables={variables}
                          />
                        </TableCell>
                        <TableCell>
                          <div className="flex gap-1">
                            <Button
                              size="icon"
                              variant="ghost"
                              onClick={() =>
                                handleOpenEditDialog(keybind, index)
                              }
                            >
                              <Pencil className="h-4 w-4" />
                            </Button>
                            <Button
                              size="icon"
                              variant="ghost"
                              onClick={() => handleDeleteKeybind(index)}
                            >
                              <Trash2 className="h-4 w-4 text-destructive" />
                            </Button>
                          </div>
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

      {/* Add/Edit Keybind Dialog */}
      <Dialog open={showDialog} onOpenChange={setShowDialog}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>
              {editingIndex !== null ? "Edit Keybind" : "Add New Keybind"}
            </DialogTitle>
            <DialogDescription>
              {editingIndex !== null
                ? "Modify the keybind settings below."
                : "Create a new keyboard shortcut for Hyprland."}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            {/* Modifiers */}
            <div className="space-y-2">
              <Label>Modifiers (optional)</Label>
              <div className="flex gap-4">
                {["SUPER", "SHIFT", "ALT", "CTRL"].map((mod) => (
                  <div key={mod} className="flex items-center space-x-2">
                    <Checkbox
                      id={`mod-${mod}`}
                      checked={formModifiers.includes(mod)}
                      onCheckedChange={() => toggleModifier(mod)}
                      disabled={formLoading}
                    />
                    <Label
                      htmlFor={`mod-${mod}`}
                      className="text-sm font-normal cursor-pointer"
                    >
                      {mod}
                    </Label>
                  </div>
                ))}
              </div>
            </div>

            {/* Key */}
            <div className="space-y-2">
              <Label htmlFor="key">Key *</Label>
              <Input
                id="key"
                placeholder="e.g., Q, Return, 1, left, F1"
                value={formKey}
                onChange={(e) => {
                  setFormKey(e.target.value);
                  setFormError(null);
                }}
                disabled={formLoading}
              />
              <p className="text-xs text-muted-foreground">
                Examples: Q, Return, Escape, left, right, up, down, F1-F12, 1-9
              </p>
            </div>

            {/* Dispatcher */}
            <div className="space-y-2">
              <Label htmlFor="dispatcher">Dispatcher (Action) *</Label>
              <Input
                id="dispatcher"
                placeholder="e.g., exec, killactive, workspace"
                value={formDispatcher}
                onChange={(e) => handleDispatcherChange(e.target.value)}
                disabled={formLoading}
                list="dispatcher-suggestions"
              />
              <datalist id="dispatcher-suggestions">
                {dispatcherSuggestions.map((d) => (
                  <option key={d} value={d}>
                    {getDispatcherDescription(d)}
                  </option>
                ))}
              </datalist>
              <p className="text-xs text-muted-foreground">
                Common: exec, killactive, workspace, togglefloating, fullscreen
              </p>
            </div>

            {/* Parameters */}
            <div className="space-y-2">
              <Label htmlFor="params">Parameters (optional)</Label>
              <Input
                id="params"
                placeholder="e.g., kitty, 1, l"
                value={formParams}
                onChange={(e) => setFormParams(e.target.value)}
                disabled={formLoading}
              />
              <p className="text-xs text-muted-foreground">
                Command arguments. Can use variables like $terminal
              </p>
            </div>

            {formError && (
              <p className="text-sm text-destructive">{formError}</p>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={handleCloseDialog}
              disabled={formLoading}
            >
              Cancel
            </Button>
            <Button onClick={handleSubmit} disabled={formLoading}>
              {formLoading
                ? editingIndex !== null
                  ? "Saving..."
                  : "Adding..."
                : editingIndex !== null
                  ? "Save Changes"
                  : "Add Keybind"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
