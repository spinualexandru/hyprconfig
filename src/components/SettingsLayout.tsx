import * as Tabs from "@radix-ui/react-tabs";
import { cn } from "../lib/utils";
import General from "../pages/General";
import Display from "../pages/Display";
import Network from "../pages/Network";
import Keybinds from "../pages/Keybinds";
import About from "../pages/About";
import {
  GearIcon,
  DesktopIcon,
  GlobeIcon,
  KeyboardIcon,
  InfoCircledIcon,
} from "@radix-ui/react-icons";

const menuItems = [
  { id: "general", label: "General", icon: GearIcon, component: General },
  { id: "display", label: "Display", icon: DesktopIcon, component: Display },
  { id: "network", label: "Network", icon: GlobeIcon, component: Network },
  { id: "keybinds", label: "Keybinds", icon: KeyboardIcon, component: Keybinds },
  { id: "about", label: "About", icon: InfoCircledIcon, component: About },
];

export default function SettingsLayout() {
  return (
    <div className="flex h-screen bg-gray-50 dark:bg-gray-900">
      <Tabs.Root defaultValue="general" orientation="vertical" className="flex w-full">
        {/* Sidebar */}
        <Tabs.List className="flex flex-col w-56 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 py-4">
          <div className="px-4 mb-4">
            <h1 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
              Settings
            </h1>
          </div>
          <nav className="flex-1 space-y-1 px-2">
            {menuItems.map((item) => {
              const Icon = item.icon;
              return (
                <Tabs.Trigger
                  key={item.id}
                  value={item.id}
                  className={cn(
                    "w-full flex items-center space-x-3 px-3 py-2 rounded-md text-sm font-medium transition-colors",
                    "text-gray-700 hover:bg-gray-100 hover:text-gray-900",
                    "dark:text-gray-300 dark:hover:bg-gray-700 dark:hover:text-gray-100",
                    "data-[state=active]:bg-blue-50 data-[state=active]:text-blue-600",
                    "dark:data-[state=active]:bg-blue-900/30 dark:data-[state=active]:text-blue-400",
                    "focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
                  )}
                >
                  <Icon className="w-5 h-5" />
                  <span>{item.label}</span>
                </Tabs.Trigger>
              );
            })}
          </nav>
        </Tabs.List>

        {/* Content Area */}
        <div className="flex-1 overflow-y-auto bg-gray-50 dark:bg-gray-900">
          {menuItems.map((item) => {
            const Component = item.component;
            return (
              <Tabs.Content key={item.id} value={item.id} className="h-full">
                <Component />
              </Tabs.Content>
            );
          })}
        </div>
      </Tabs.Root>
    </div>
  );
}
