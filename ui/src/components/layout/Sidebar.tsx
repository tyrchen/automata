import React from 'react';
import { NavLink, useLocation } from 'react-router-dom';
import { cn } from '../../lib/utils';
import { Button } from '../ui/button';
import { Separator } from '../ui/separator';
import {
  LayoutDashboard,
  Workflow,
  Play,
  Settings,
  Plus,
  GitBranch,
  Activity
} from 'lucide-react';

interface NavItem {
  to: string;
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  description?: string;
}

const navigation: NavItem[] = [
  {
    to: '/',
    icon: LayoutDashboard,
    label: 'Dashboard',
    description: 'Overview and metrics'
  },
  {
    to: '/workflows',
    icon: Workflow,
    label: 'Workflows',
    description: 'Manage workflows'
  },
  {
    to: '/executions',
    icon: Activity,
    label: 'Executions',
    description: 'View execution history'
  },
];

const quickActions: NavItem[] = [
  {
    to: '/workflows/new',
    icon: Plus,
    label: 'New Workflow',
    description: 'Create a new workflow'
  },
];

export const Sidebar: React.FC = () => {
  const location = useLocation();

  return (
    <div className="w-64 bg-background border-r border-border flex flex-col">
      {/* Logo/Brand */}
      <div className="p-6 border-b border-border">
        <div className="flex items-center gap-2">
          <GitBranch className="h-8 w-8 text-primary" />
          <div>
            <h1 className="text-xl font-bold">Automata</h1>
            <p className="text-xs text-muted-foreground">Workflow Engine</p>
          </div>
        </div>
      </div>

      {/* Quick Actions */}
      <div className="p-4">
        <div className="space-y-2">
          {quickActions.map((item) => {
            const Icon = item.icon;
            const isActive = location.pathname === item.to;

            return (
              <Button
                key={item.to}
                asChild
                variant={isActive ? "default" : "outline"}
                className="w-full justify-start gap-2"
              >
                <NavLink to={item.to}>
                  <Icon className="h-4 w-4" />
                  {item.label}
                </NavLink>
              </Button>
            );
          })}
        </div>
      </div>

      <Separator />

      {/* Navigation */}
      <nav className="flex-1 p-4">
        <div className="space-y-1">
          {navigation.map((item) => {
            const Icon = item.icon;
            const isActive =
              item.to === '/'
                ? location.pathname === '/'
                : location.pathname.startsWith(item.to);

            return (
              <NavLink
                key={item.to}
                to={item.to}
                className={({ isActive: linkActive }) =>
                  cn(
                    'flex items-center gap-3 px-3 py-2 text-sm rounded-lg transition-colors',
                    'hover:bg-accent hover:text-accent-foreground',
                    (isActive || linkActive)
                      ? 'bg-accent text-accent-foreground font-medium'
                      : 'text-muted-foreground'
                  )
                }
              >
                <Icon className="h-4 w-4" />
                <div className="flex-1">
                  <div>{item.label}</div>
                  {item.description && (
                    <div className="text-xs opacity-70">{item.description}</div>
                  )}
                </div>
              </NavLink>
            );
          })}
        </div>
      </nav>

      {/* Footer */}
      <div className="p-4 border-t border-border">
        <Button
          variant="ghost"
          className="w-full justify-start gap-2"
          asChild
        >
          <NavLink to="/settings">
            <Settings className="h-4 w-4" />
            Settings
          </NavLink>
        </Button>
      </div>
    </div>
  );
};
