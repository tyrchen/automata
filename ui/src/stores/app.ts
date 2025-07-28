import { create } from 'zustand';
import { devtools, persist } from 'zustand/middleware';

interface AppState {
  // Theme and UI preferences
  theme: 'light' | 'dark' | 'system';
  isDarkMode: boolean;
  sidebarCollapsed: boolean;

  // Layout state
  activeView: 'dashboard' | 'workflows' | 'executions' | 'settings';

  // User preferences
  preferences: {
    autoSave: boolean;
    showMinimap: boolean;
    snapToGrid: boolean;
    gridSize: number;
    defaultZoom: number;
    animateTransitions: boolean;
  };

  // Notification state
  notifications: Array<{
    id: string;
    type: 'info' | 'success' | 'warning' | 'error';
    title: string;
    message: string;
    timestamp: number;
    duration?: number;
  }>;

  // Actions
  setTheme: (theme: 'light' | 'dark' | 'system') => void;
  setIsDarkMode: (isDark: boolean) => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setActiveView: (view: AppState['activeView']) => void;
  updatePreferences: (preferences: Partial<AppState['preferences']>) => void;
  addNotification: (notification: Omit<AppState['notifications'][0], 'id' | 'timestamp'>) => void;
  removeNotification: (id: string) => void;
  clearNotifications: () => void;
}

export const useAppStore = create<AppState>()(
  devtools(
    persist(
      (set, get) => ({
        // Initial state
        theme: 'system',
        isDarkMode: false,
        sidebarCollapsed: false,
        activeView: 'dashboard',
        preferences: {
          autoSave: true,
          showMinimap: true,
          snapToGrid: true,
          gridSize: 20,
          defaultZoom: 1,
          animateTransitions: true,
        },
        notifications: [],

        // Actions
        setTheme: (theme) => set({ theme }),

        setIsDarkMode: (isDark) => set({ isDarkMode: isDark }),

        setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),

        setActiveView: (view) => set({ activeView: view }),

        updatePreferences: (newPreferences) =>
          set((state) => ({
            preferences: { ...state.preferences, ...newPreferences },
          })),

        addNotification: (notification) => {
          const id = `notification_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
          const newNotification = {
            ...notification,
            id,
            timestamp: Date.now(),
          };

          set((state) => ({
            notifications: [newNotification, ...state.notifications].slice(0, 10), // Keep only last 10
          }));

          // Auto-remove notification after duration
          if (notification.duration) {
            setTimeout(() => {
              set((state) => ({
                notifications: state.notifications.filter((n) => n.id !== id),
              }));
            }, notification.duration);
          }
        },

        removeNotification: (id) =>
          set((state) => ({
            notifications: state.notifications.filter((n) => n.id !== id),
          })),

        clearNotifications: () => set({ notifications: [] }),
      }),
      {
        name: 'app-store',
        partialize: (state) => ({
          theme: state.theme,
          sidebarCollapsed: state.sidebarCollapsed,
          preferences: state.preferences,
        }),
      }
    ),
    {
      name: 'app-store',
    }
  )
);
