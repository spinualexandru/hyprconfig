import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";

export default function General() {
	return (
		<div className="p-6 space-y-6">
			<div>
				<h1 className="text-3xl font-bold tracking-tight">General</h1>
				<p className="text-muted-foreground mt-2">
					General settings and preferences for Hyprland configuration.
				</p>
			</div>

			<Card>
				<CardHeader>
					<CardTitle>System Preferences</CardTitle>
					<CardDescription>Configure your system settings</CardDescription>
				</CardHeader>
				<CardContent>
					<p className="text-sm text-muted-foreground">
						General configuration options will be added here.
					</p>
				</CardContent>
			</Card>
		</div>
	);
}
