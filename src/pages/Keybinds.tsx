import { invoke } from "@tauri-apps/api/core";
import { Keyboard, RefreshCw, Plus } from "lucide-react";
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
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { TooltipProvider } from "@/components/ui/tooltip";
import type { Keybind } from "@/types/keybinds";
import type { Variable } from "@/types/variables";
import {
  KeybindsTableSkeleton,
  KeybindTableRow,
  KeybindFormDialog,
} from "@/components/keybinds";

export default function Keybinds() {
  const [keybinds, setKeybinds] = useState<Keybind[]>([]);
  const [cachedKeybinds, setCachedKeybinds] = useState<Keybind[]>([]);
  const [variables, setVariables] = useState<Variable[]>([]);
  const [loading, setLoading] = useState(false);
  const [initialLoad, setInitialLoad] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Dialog state
  const [showDialog, setShowDialog] = useState(false);
  const [editingKeybind, setEditingKeybind] = useState<{
    keybind: Keybind;
    index: number;
  } | null>(null);

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

  const handleOpenAddDialog = () => {
    setEditingKeybind(null);
    setShowDialog(true);
  };

  const handleOpenEditDialog = (keybind: Keybind, index: number) => {
    setEditingKeybind({ keybind, index });
    setShowDialog(true);
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
                      <KeybindTableRow
                        key={index}
                        keybind={keybind}
                        index={index}
                        variables={variables}
                        onEdit={handleOpenEditDialog}
                        onDelete={handleDeleteKeybind}
                      />
                    ))}
                  </TableBody>
                </Table>
              </TooltipProvider>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Add/Edit Keybind Dialog */}
      <KeybindFormDialog
        open={showDialog}
        onOpenChange={setShowDialog}
        editingKeybind={editingKeybind}
        onSuccess={loadKeybinds}
      />
    </div>
  );
}
