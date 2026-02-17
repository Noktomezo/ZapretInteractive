import { Link, useLocation } from "@tanstack/react-router";
import {
  Home,
  Layers,
  FileCode,
  Settings,
  PanelRightOpen,
  Filter,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useSidebarStore } from "@/stores/sidebar.store";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

const navItems = [
  { path: "/", label: "Главная", icon: Home },
  { path: "/strategies", label: "Стратегии", icon: Layers },
  { path: "/filters", label: "Фильтры", icon: Filter },
  { path: "/placeholders", label: "Плейсхолдеры", icon: FileCode },
  { path: "/settings", label: "Настройки", icon: Settings },
];

export function Sidebar() {
  const location = useLocation();
  const currentPath = location.pathname;
  const { collapsed, toggle } = useSidebarStore();

  return (
    <aside
      className={cn(
        "border-r border-border bg-card flex flex-col transition-all duration-200",
        collapsed ? "w-16" : "w-64",
      )}
    >
      <div className={cn("p-2 border-b border-border flex", collapsed ? "justify-center" : "justify-end")}>
        <button
          onClick={toggle}
          className="cursor-pointer hover:opacity-80 transition-opacity p-2"
          title={collapsed ? "Развернуть" : "Свернуть"}
        >
          <PanelRightOpen className={cn("w-4 h-4 text-muted-foreground", collapsed && "rotate-180")} />
        </button>
      </div>

      <nav className="flex-1 p-2">
        <ul className="space-y-1">
          {navItems.map((item) => {
            const isActive = currentPath === item.path;
            const Icon = item.icon;
            const linkContent = (
              <Link
                to={item.path}
                className={cn(
                  "cursor-pointer flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium",
                  "transition-all duration-200 ease-out",
                  isActive
                    ? "bg-primary text-primary-foreground scale-[1.02] shadow-sm"
                    : "text-muted-foreground hover:bg-accent hover:text-accent-foreground hover:scale-[1.01]",
                  collapsed && "justify-center px-0",
                )}
              >
                <Icon className="w-4 h-4 shrink-0" />
                {!collapsed && item.label}
              </Link>
            );

            return (
              <li key={item.path}>
                {collapsed ? (
                  <Tooltip>
                    <TooltipTrigger asChild>{linkContent}</TooltipTrigger>
                    <TooltipContent side="right">{item.label}</TooltipContent>
                  </Tooltip>
                ) : (
                  linkContent
                )}
              </li>
            );
          })}
        </ul>
      </nav>
    </aside>
  );
}
