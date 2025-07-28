import React from 'react';
import { useLocation } from 'react-router-dom';
import { Button } from '../ui/button';
import { Badge } from '../ui/badge';
import {
  Bell,
  User,
  Sun,
  Moon,
  MoreHorizontal,
  RefreshCw
} from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu';
import { useAppStore } from '../../stores';

const getPageTitle = (pathname: string): { title: string; subtitle?: string } => {
  if (pathname === '/') {
    return { title: 'Dashboard', subtitle: 'Workflow overview and metrics' };
  }
  if (pathname.startsWith('/workflows/new')) {
    return { title: 'New Workflow', subtitle: 'Create a new workflow' };
  }
  if (pathname.startsWith('/workflows/') && pathname.includes('/edit')) {
    return { title: 'Edit Workflow', subtitle: 'Modify existing workflow' };
  }
  if (pathname.startsWith('/workflows')) {
    return { title: 'Workflows', subtitle: 'Manage your workflows' };
  }
  if (pathname.startsWith('/executions')) {
    return { title: 'Executions', subtitle: 'View execution history' };
  }
  if (pathname.startsWith('/settings')) {
    return { title: 'Settings', subtitle: 'Application preferences' };
  }

  return { title: 'Automata' };
};

export const Header: React.FC = () => {
  const location = useLocation();
  const { isDarkMode, setIsDarkMode } = useAppStore();
  const { title, subtitle } = getPageTitle(location.pathname);

  return (
    <header className="h-16 border-b border-border bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="flex h-full items-center justify-between px-6">
        {/* Page Title */}
        <div className="flex items-center gap-4">
          <div>
            <h2 className="text-lg font-semibold">{title}</h2>
            {subtitle && (
              <p className="text-sm text-muted-foreground">{subtitle}</p>
            )}
          </div>
        </div>

        {/* Actions */}
        <div className="flex items-center gap-2">
          {/* Refresh */}
          <Button variant="ghost" size="sm">
            <RefreshCw className="h-4 w-4" />
          </Button>

          {/* Notifications */}
          <Button variant="ghost" size="sm" className="relative">
            <Bell className="h-4 w-4" />
            <Badge
              variant="destructive"
              className="absolute -top-1 -right-1 h-5 w-5 flex items-center justify-center p-0 text-xs"
            >
              3
            </Badge>
          </Button>

          {/* Theme Toggle */}
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setIsDarkMode(!isDarkMode)}
          >
            {isDarkMode ? (
              <Sun className="h-4 w-4" />
            ) : (
              <Moon className="h-4 w-4" />
            )}
          </Button>

          {/* User Menu */}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="sm">
                <User className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-56">
              <div className="flex items-center justify-start gap-2 p-2">
                <div className="flex flex-col space-y-1 leading-none">
                  <p className="font-medium">John Doe</p>
                  <p className="text-xs text-muted-foreground">
                    john.doe@example.com
                  </p>
                </div>
              </div>
              <DropdownMenuSeparator />
              <DropdownMenuItem>
                Profile
              </DropdownMenuItem>
              <DropdownMenuItem>
                Settings
              </DropdownMenuItem>
              <DropdownMenuItem>
                Keyboard shortcuts
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem>
                Log out
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
    </header>
  );
};
