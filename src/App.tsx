import { useState } from "react";
import { AppSidebar } from "@/components/app-sidebar";
import { SidebarProvider, SidebarInset, SidebarTrigger } from "@/components/ui/sidebar";
import { Separator } from "@/components/ui/separator";
import General from "@/pages/General";
import Network from "@/pages/Network";
import Displays from "@/pages/Displays";
import About from "@/pages/About";

function App() {
  const [currentPage, setCurrentPage] = useState("general");

  const renderPage = () => {
    switch (currentPage) {
      case "general":
        return <General />;
      case "network":
        return <Network />;
      case "displays":
        return <Displays />;
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
        <main className="flex-1 overflow-y-auto">
          {renderPage()}
        </main>
      </SidebarInset>
    </SidebarProvider>
  );
}

export default App;
