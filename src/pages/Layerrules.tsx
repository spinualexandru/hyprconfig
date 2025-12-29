import { invoke } from "@tauri-apps/api/core";
import { Layers, RefreshCw, Trash2, ChevronDown, ChevronRight } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import type { Layerrule } from "@/types/layerrules";

function LayerrulesTableSkeleton() {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
            <Layers className="h-5 w-5 text-primary" />
          </div>
          <div>
            <CardTitle>Layerrules</CardTitle>
            <CardDescription>
              Your Hyprland layerrule configurations
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {[...Array(4)].map((_, i) => (
            <div key={i} className="flex items-center gap-4">
              <Skeleton className="h-6 w-40" />
              <Skeleton className="h-6 flex-1" />
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

interface LayerruleCardProps {
  name: string;
  onDelete: (name: string) => void;
}

function LayerruleCard({ name, onDelete }: LayerruleCardProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [rule, setRule] = useState<Layerrule | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadRule = async () => {
    if (rule) return; // Already loaded
    setLoading(true);
    setError(null);

    invoke<Layerrule>("get_layerrule", { name })
      .then((result) => {
        setRule(result);
      })
      .catch((err) => {
        setError(err as string);
      })
      .finally(() => {
        setLoading(false);
      });
  };

  const handleToggle = () => {
    if (!isOpen) {
      loadRule();
    }
    setIsOpen(!isOpen);
  };

  return (
    <div>
      <div className={`flex items-center justify-between p-4 border ${isOpen ? "rounded-t-lg" : "rounded-lg"}`}>
        <Button
          variant="ghost"
          className="flex items-center gap-2 p-0 h-auto hover:bg-transparent"
          onClick={handleToggle}
        >
          {isOpen ? (
            <ChevronDown className="h-4 w-4" />
          ) : (
            <ChevronRight className="h-4 w-4" />
          )}
          <code className="text-sm font-mono">{name}</code>
        </Button>
        <Button
          size="icon"
          variant="ghost"
          onClick={() => onDelete(name)}
        >
          <Trash2 className="h-4 w-4 text-destructive" />
        </Button>
      </div>
      {isOpen && (
        <div className="p-4 pt-2 border-x border-b rounded-b-lg">
          {loading ? (
            <div className="space-y-2">
              <Skeleton className="h-4 w-full" />
              <Skeleton className="h-4 w-3/4" />
            </div>
          ) : error ? (
            <p className="text-sm text-destructive">{error}</p>
          ) : rule ? (
            <div className="space-y-4">
              {/* Match Properties */}
              {rule.match_properties.length > 0 && (
                <div>
                  <h4 className="text-sm font-medium text-muted-foreground mb-2">
                    Match Conditions
                  </h4>
                  <div className="flex flex-wrap gap-2">
                    {rule.match_properties.map((prop, i) => (
                      <Badge key={i} variant="secondary">
                        <span className="font-mono text-xs">
                          {prop.key}: {prop.value}
                        </span>
                      </Badge>
                    ))}
                  </div>
                </div>
              )}
              {/* Effect Properties */}
              {rule.effect_properties.length > 0 && (
                <div>
                  <h4 className="text-sm font-medium text-muted-foreground mb-2">
                    Effects
                  </h4>
                  <div className="flex flex-wrap gap-2">
                    {rule.effect_properties.map((prop, i) => (
                      <Badge key={i} variant="outline">
                        <span className="font-mono text-xs">
                          {prop.key}: {prop.value}
                        </span>
                      </Badge>
                    ))}
                  </div>
                </div>
              )}
              {rule.match_properties.length === 0 &&
                rule.effect_properties.length === 0 && (
                  <p className="text-sm text-muted-foreground">
                    No properties defined
                  </p>
                )}
            </div>
          ) : null}
        </div>
      )}
    </div>
  );
}

export default function Layerrules() {
  const [ruleNames, setRuleNames] = useState<string[]>([]);
  const [cachedRuleNames, setCachedRuleNames] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [initialLoad, setInitialLoad] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadLayerrules();
  }, []);

  const loadLayerrules = async () => {
    setLoading(true);
    setError(null);

    invoke<string[]>("get_layerrule_names")
      .then((result) => {
        setRuleNames(result);
        setCachedRuleNames(result);
      })
      .catch((err) => {
        setError(err as string);
        if (cachedRuleNames.length === 0) {
          setRuleNames([]);
        }
      })
      .finally(() => {
        setLoading(false);
        setInitialLoad(false);
      });
  };

  const handleDeleteLayerrule = async (name: string) => {
    invoke("delete_layerrule", { name })
      .then(() => {
        loadLayerrules();
      })
      .catch((err) => {
        setError(err as string);
      });
  };

  const displayRuleNames =
    loading && cachedRuleNames.length > 0 ? cachedRuleNames : ruleNames;

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight text-foreground">
            Layerrules
          </h1>
          <p className="text-muted-foreground mt-2">
            View and manage your Hyprland v2 layerrule configurations
          </p>
        </div>
        <div className="flex gap-2">
          <Button onClick={loadLayerrules} variant="outline" disabled={loading}>
            <RefreshCw
              className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`}
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

      {/* Layerrules Display */}
      {(loading && cachedRuleNames.length === 0) || initialLoad ? (
        <LayerrulesTableSkeleton />
      ) : displayRuleNames.length === 0 ? (
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">
              No layerrules found in your Hyprland configuration.
            </p>
            <p className="text-sm text-muted-foreground mt-2">
              Layerrules v2 use the special category syntax:{" "}
              <code className="bg-muted px-1 rounded">
                layerrule[name] {"{"} ... {"}"}
              </code>
            </p>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardHeader>
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
                <Layers className="h-5 w-5 text-primary" />
              </div>
              <div>
                <CardTitle>Layerrules ({displayRuleNames.length})</CardTitle>
                <CardDescription>
                  Your Hyprland layerrule configurations
                </CardDescription>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {displayRuleNames.map((name) => (
                <LayerruleCard
                  key={name}
                  name={name}
                  onDelete={handleDeleteLayerrule}
                />
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
