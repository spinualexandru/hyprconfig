import { useEffect, useState } from "react";
import { AppSidebar } from "@/components/app-sidebar";
import { Separator } from "@/components/ui/separator";
import {
	SidebarInset,
	SidebarProvider,
	SidebarTrigger,
} from "@/components/ui/sidebar";
import About from "@/pages/About";
import Appearance from "@/pages/Appearance";
import Audio from "@/pages/Audio";
import Displays from "@/pages/Displays";
import General from "@/pages/General";
import Keybinds from "@/pages/Keybinds";
import Layerrules from "@/pages/Layerrules";
import Network from "@/pages/Network";
import Variables from "@/pages/Variables";
import EnvVars from "@/pages/EnvVars";
import Windowrules from "@/pages/Windowrules";
import { loadTheme } from "@/lib/theme-loader";

function App() {
	const [currentPage, setCurrentPage] = useState("general");

	// Load theme CSS on mount
	useEffect(() => {
		loadTheme();
	}, []);

	const renderPage = () => {
		switch (currentPage) {
			case "general":
				return <General />;
			case "appearance":
				return <Appearance />;
			case "network":
				return <Network />;
			case "audio":
				return <Audio />;
			case "displays":
				return <Displays />;
			case "keybinds":
				return <Keybinds />;
			case "variables":
				return <Variables />;
			case "envvars":
				return <EnvVars />;
			case "windowrules":
				return <Windowrules />;
			case "layerrules":
				return <Layerrules />;
			case "about":
				return <About />;
			default:
				return <General />;
		}
	};

	return (
		<SidebarProvider defaultOpen={true}>
			<AppSidebar currentPage={currentPage} onNavigate={setCurrentPage} />
			<SidebarInset>
				<header className="flex h-16 shrink-0 items-center gap-2 border-b px-4">
					<SidebarTrigger className="-ml-1" />
					<Separator orientation="vertical" className="mr-2 h-4" />
				</header>
				<main className="flex-1 overflow-y-auto">{renderPage()}</main>
			</SidebarInset>
		</SidebarProvider>
	);
}

export default App;
