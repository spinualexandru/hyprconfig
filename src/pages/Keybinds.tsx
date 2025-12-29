import { invoke } from "@tauri-apps/api/core";
import { Keyboard, RefreshCw, Plus, Globe } from "lucide-react";
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
import { TooltipProvider } from "@/components/ui/tooltip";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { Keybind } from "@/types/keybinds";
import type { Variable } from "@/types/variables";
import {
  KeybindsTableSkeleton,
  KeybindFormDialog,
  createKeybindColumns,
} from "@/components/keybinds";

export default function Keybinds() {
  const [keybinds, setKeybinds] = useState<Keybind[]>([]);
  const [cachedKeybinds, setCachedKeybinds] = useState<Keybind[]>([]);
  const [bindus, setBindus] = useState<Keybind[]>([]);
  const [cachedBindus, setCachedBindus] = useState<Keybind[]>([]);
  const [variables, setVariables] = useState<Variable[]>([]);
  const [loading, setLoading] = useState(false);
  const [binduLoading, setBinduLoading] = useState(false);
  const [initialLoad, setInitialLoad] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<"bind" | "bindu">("bind");

  // Dialog state
  const [showDialog, setShowDialog] = useState(false);
  const [showBinduDialog, setShowBinduDialog] = useState(false);
  const [editingKeybind, setEditingKeybind] = useState<{
    keybind: Keybind;
    index: number;
  } | null>(null);

  useEffect(() => {
    loadKeybinds();
    loadBindus();
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

  // ==================== BINDU HANDLERS ====================

  const loadBindus = async () => {
    setBinduLoading(true);

    invoke<Keybind[]>("get_all_bindu")
      .then((result) => {
        setBindus(result);
        setCachedBindus(result);
      })
      .catch((err) => {
        console.error("Failed to load bindus:", err);
        if (cachedBindus.length === 0) {
          setBindus([]);
        }
      })
      .finally(() => {
        setBinduLoading(false);
      });
  };

  const handleOpenAddBinduDialog = () => {
    setEditingKeybind(null);
    setShowBinduDialog(true);
  };

  const handleDeleteBindu = async (index: number) => {
    invoke("delete_bindu", { index })
      .then(() => {
        loadBindus();
      })
      .catch((err) => {
        setError(err as string);
      });
  };

  // Use cached keybinds if loading and cache exists, otherwise use current keybinds
  const displayKeybinds =
    loading && cachedKeybinds.length > 0 ? cachedKeybinds : keybinds;

  const columns = useMemo(
    () =>
      createKeybindColumns({
        variables,
        onEdit: handleOpenEditDialog,
        onDelete: handleDeleteKeybind,
      }),
    [variables],
  );

  // Bindu display and columns
  const displayBindus =
    binduLoading && cachedBindus.length > 0 ? cachedBindus : bindus;

  const binduColumns = useMemo(
    () =>
      createKeybindColumns({
        variables,
        // bindu doesn't support edit, only add/delete
        onDelete: handleDeleteBindu,
      }),
    [variables],
  );

  // Current loading state based on active tab
  const isLoading = activeTab === "bind" ? loading : binduLoading;

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight text-foreground">Keybinds</h1>
          <p className="text-muted-foreground mt-2">
            View and change your Hyprland keyboard shortcuts and keybindings
          </p>
        </div>
        <div className="flex gap-2">
          <Button
            onClick={activeTab === "bind" ? handleOpenAddDialog : handleOpenAddBinduDialog}
            variant="default"
          >
            <Plus className="h-4 w-4 mr-2" />
            {activeTab === "bind" ? "Add Keybind" : "Add Universal Bind"}
          </Button>
          <Button
            onClick={activeTab === "bind" ? loadKeybinds : loadBindus}
            variant="outline"
            disabled={isLoading}
          >
            <RefreshCw
              className={`h-4 w-4 mr-2 ${isLoading ? "animate-spin" : ""}`}
            />
            Refresh
          </Button>
        </div>
      </div>

      {/* Error Display */}
      {error && (
        <Card className="border-destructive">
          <CardHeader>
            <CardTitle className="text-destructive">Error</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground">{error}</p>
          </CardContent>
        </Card>
      )}

      <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as "bind" | "bindu")}>
        <TabsList>
          <TabsTrigger value="bind" className="flex items-center gap-2">
            <Keyboard className="h-4 w-4" />
            Keybinds
          </TabsTrigger>
          <TabsTrigger value="bindu" className="flex items-center gap-2">
            <Globe className="h-4 w-4" />
            Universal
          </TabsTrigger>
        </TabsList>

        {/* Regular Keybinds Tab */}
        <TabsContent value="bind" className="mt-4">
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
                    <DataTable columns={columns} data={displayKeybinds} />
                  </TooltipProvider>
                </div>
              </CardContent>
            </Card>
          )}
        </TabsContent>

        {/* Universal Binds (bindu) Tab */}
        <TabsContent value="bindu" className="mt-4">
          {binduLoading && cachedBindus.length === 0 ? (
            <KeybindsTableSkeleton />
          ) : displayBindus.length === 0 ? (
            <Card>
              <CardContent className="pt-6">
                <p className="text-sm text-muted-foreground">
                  No universal binds found. Universal binds work across all submaps.
                </p>
              </CardContent>
            </Card>
          ) : (
            <Card>
              <CardHeader>
                <div className="flex items-center gap-3">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
                    <Globe className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <CardTitle>Universal Binds ({displayBindus.length})</CardTitle>
                    <CardDescription>
                      Keybinds that work across all submaps
                    </CardDescription>
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                <div className="rounded-md border">
                  <TooltipProvider>
                    <DataTable columns={binduColumns} data={displayBindus} />
                  </TooltipProvider>
                </div>
              </CardContent>
            </Card>
          )}
        </TabsContent>
      </Tabs>

      {/* Add/Edit Keybind Dialog */}
      <KeybindFormDialog
        open={showDialog}
        onOpenChange={setShowDialog}
        editingKeybind={editingKeybind}
        onSuccess={loadKeybinds}
      />

      {/* Add/Edit Universal Bind Dialog */}
      <KeybindFormDialog
        open={showBinduDialog}
        onOpenChange={setShowBinduDialog}
        editingKeybind={editingKeybind}
        onSuccess={loadBindus}
        isBindu
      />
    </div>
  );
}
