import { useState } from "react";
import Sidebar from "./components/Sidebar";
import General from "./pages/General";
import Network from "./pages/Network";
import Displays from "./pages/Displays";
import About from "./pages/About";

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
    <div className="flex h-screen bg-gray-100 dark:bg-gray-950">
      <Sidebar currentPage={currentPage} onNavigate={setCurrentPage} />
      <main className="flex-1 overflow-y-auto">
        {renderPage()}
      </main>
    </div>
  );
}

export default App;
