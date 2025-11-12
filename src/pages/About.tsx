import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export default function About() {
  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">About</h1>
        <p className="text-muted-foreground mt-2">
          Information about Hyprconfig
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Hyprconfig</CardTitle>
          <CardDescription>Version 0.1.0</CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            A GUI application for configuring Hyprland compositor built with Tauri 2.0.
          </p>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Technology Stack</CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="space-y-2 text-sm text-muted-foreground">
            <li>• Frontend: React 18 + TypeScript</li>
            <li>• Backend: Rust + Tauri 2.0</li>
            <li>• UI: Tailwind CSS + shadcn/ui</li>
            <li>• Hyprland Integration: hyprland crate</li>
          </ul>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>License</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">MIT License</p>
        </CardContent>
      </Card>
    </div>
  );
}
