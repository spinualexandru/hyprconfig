import { AppWindow, DollarSign, Info, Keyboard, Layers, Monitor, Moon, Palette, Settings, Sun, Volume2, Wifi } from "lucide-react";
import type * as React from "react";
import { Separator } from "@/components/ui/separator";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import { useTheme } from "@/hooks/use-theme";
import HyprlandIcon from "./icons/hyprland";

const navItems = [
  {
    title: "General",
    id: "general",
    icon: Settings,
  },
  {
    title: "Appearance",
    id: "appearance",
    icon: Palette,
  },
  {
    title: "Network",
    id: "network",
    icon: Wifi,
  },
  {
    title: "Audio",
    id: "audio",
    icon: Volume2,
  },
  {
    title: "Displays",
    id: "displays",
    icon: Monitor,
  },
  {
    title: "Keybinds",
    id: "keybinds",
    icon: Keyboard,
  },
  {
    title: "Variables",
    id: "variables",
    icon: DollarSign,
  },
  {
    title: "Windowrules",
    id: "windowrules",
    icon: AppWindow,
  },
  {
    title: "Layerrules",
    id: "layerrules",
    icon: Layers,
  },
  {
    title: "About",
    id: "about",
    icon: Info,
  },
];

interface AppSidebarProps extends React.ComponentProps<typeof Sidebar> {
  currentPage: string;
  onNavigate: (page: string) => void;
}

export function AppSidebar({
  currentPage,
  onNavigate,
  ...props
}: AppSidebarProps) {
  const { theme, toggleTheme } = useTheme();

  return (
    <Sidebar collapsible="icon" {...props}>
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              size="lg"
              className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
            >
              <div className="flex aspect-square size-8 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground">
                <HyprlandIcon className="size-4 fill-background" />
              </div>
              <div className="grid flex-1 text-left text-sm leading-tight">
                <span className="truncate font-semibold">Hyprconfig</span>
                <span className="truncate text-xs">Hyprland Settings</span>
              </div>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent>
        <SidebarMenu className="px-2">
          {navItems.map((item) => {
            const Icon = item.icon;
            const isActive = currentPage === item.id;

            return (
              <SidebarMenuItem key={item.id}>
                <SidebarMenuButton
                  onClick={() => onNavigate(item.id)}
                  isActive={isActive}
                  tooltip={item.title}
                  className="px-2 justify-start"
                >
                  <Icon className="size-4 shrink-0" />
                  <span>{item.title}</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            );
          })}
        </SidebarMenu>
      </SidebarContent>
      <SidebarFooter className="mt-auto">
        <Separator className="mb-2" />
        <SidebarMenu className="px-2">
          <SidebarMenuItem>
            <SidebarMenuButton
              onClick={toggleTheme}
              tooltip={
                theme === "light"
                  ? "Switch to dark mode"
                  : "Switch to light mode"
              }
              className="px-2 justify-start"
            >
              {theme === "light" ? (
                <Sun className="size-4 shrink-0" />
              ) : (
                <Moon className="size-4 shrink-0" />
              )}
              <span>{theme === "light" ? "Light Mode" : "Dark Mode"}</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  );
}
