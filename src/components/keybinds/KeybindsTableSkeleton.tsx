import { Keyboard } from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";

export function KeybindsTableSkeleton() {
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
