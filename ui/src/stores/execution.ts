import { create } from 'zustand';
import { devtools } from 'zustand/middleware';
import {
  ExecutionListItem,
  ExecutionStatus,
  ExecutionUpdate,
  GetExecutionResponse,
  ApiError,
} from '../types';

interface ExecutionState {
  // List of executions
  executions: ExecutionListItem[];

  // Current execution being monitored
  currentExecution: GetExecutionResponse | null;

  // Real-time execution updates
  executionUpdates: Record<string, ExecutionUpdate>;

  // Execution filters and sorting
  filters: {
    status?: ExecutionStatus;
    workflowId?: string;
    dateRange?: { start: string; end: string };
  };
  sortBy: 'started_at' | 'duration_ms' | 'status';
  sortOrder: 'asc' | 'desc';

  // UI state
  isLoading: boolean;
  isExecuting: boolean;
  selectedExecutionId: string | null;
  showExecutionDetails: boolean;
  error: ApiError | null;

  // WebSocket connection state
  isConnected: boolean;
  connectionError: string | null;

  // Actions
  setExecutions: (executions: ExecutionListItem[]) => void;
  addExecution: (execution: ExecutionListItem) => void;
  updateExecution: (executionId: string, updates: Partial<ExecutionListItem>) => void;
  removeExecution: (executionId: string) => void;
  setCurrentExecution: (execution: GetExecutionResponse | null) => void;
  setExecutionUpdate: (update: ExecutionUpdate) => void;
  setFilters: (filters: Partial<ExecutionState['filters']>) => void;
  setSortBy: (sortBy: ExecutionState['sortBy']) => void;
  setSortOrder: (order: 'asc' | 'desc') => void;
  setSelectedExecutionId: (executionId: string | null) => void;
  setShowExecutionDetails: (show: boolean) => void;
  setIsLoading: (loading: boolean) => void;
  setIsExecuting: (executing: boolean) => void;
  setError: (error: ApiError | null) => void;
  setIsConnected: (connected: boolean) => void;
  setConnectionError: (error: string | null) => void;
  clearExecutions: () => void;

  // Computed getters
  getFilteredExecutions: () => ExecutionListItem[];
  getExecutionById: (executionId: string) => ExecutionListItem | null;
  getExecutionsByWorkflow: (workflowId: string) => ExecutionListItem[];
  getExecutionStats: () => {
    total: number;
    completed: number;
    failed: number;
    running: number;
    pending: number;
  };
}

export const useExecutionStore = create<ExecutionState>()(
  devtools(
    (set, get) => ({
      // Initial state
      executions: [],
      currentExecution: null,
      executionUpdates: {},
      filters: {},
      sortBy: 'started_at',
      sortOrder: 'desc',
      isLoading: false,
      isExecuting: false,
      selectedExecutionId: null,
      showExecutionDetails: false,
      error: null,
      isConnected: false,
      connectionError: null,

      // Actions
      setExecutions: (executions) => set({ executions }),

      addExecution: (execution) =>
        set((state) => ({
          executions: [execution, ...state.executions],
        })),

      updateExecution: (executionId, updates) =>
        set((state) => ({
          executions: state.executions.map((exec) =>
            exec.execution_id === executionId ? { ...exec, ...updates } : exec
          ),
        })),

      removeExecution: (executionId) =>
        set((state) => ({
          executions: state.executions.filter(
            (exec) => exec.execution_id !== executionId
          ),
          selectedExecutionId:
            state.selectedExecutionId === executionId
              ? null
              : state.selectedExecutionId,
        })),

      setCurrentExecution: (execution) => set({ currentExecution: execution }),

      setExecutionUpdate: (update) =>
        set((state) => ({
          executionUpdates: {
            ...state.executionUpdates,
            [update.execution_id]: update,
          },
        })),

      setFilters: (filters) =>
        set((state) => ({
          filters: { ...state.filters, ...filters },
        })),

      setSortBy: (sortBy) => set({ sortBy }),

      setSortOrder: (sortOrder) => set({ sortOrder }),

      setSelectedExecutionId: (executionId) =>
        set({ selectedExecutionId: executionId }),

      setShowExecutionDetails: (show) => set({ showExecutionDetails: show }),

      setIsLoading: (loading) => set({ isLoading: loading }),

      setIsExecuting: (executing) => set({ isExecuting: executing }),

      setError: (error) => set({ error }),

      setIsConnected: (connected) => set({ isConnected: connected }),

      setConnectionError: (error) => set({ connectionError: error }),

      clearExecutions: () =>
        set({
          executions: [],
          currentExecution: null,
          executionUpdates: {},
          selectedExecutionId: null,
          error: null,
        }),

      // Computed getters
      getFilteredExecutions: () => {
        const { executions, filters, sortBy, sortOrder } = get();

        let filtered = [...executions];

        // Apply filters
        if (filters.status) {
          filtered = filtered.filter((exec) => exec.status === filters.status);
        }

        if (filters.workflowId) {
          filtered = filtered.filter((exec) => exec.workflow_id === filters.workflowId);
        }

        if (filters.dateRange) {
          const { start, end } = filters.dateRange;
          filtered = filtered.filter((exec) => {
            const execDate = new Date(exec.started_at);
            return execDate >= new Date(start) && execDate <= new Date(end);
          });
        }

        // Apply sorting
        filtered.sort((a, b) => {
          let aValue: any = a[sortBy];
          let bValue: any = b[sortBy];

          if (sortBy === 'started_at') {
            aValue = new Date(aValue).getTime();
            bValue = new Date(bValue).getTime();
          }

          const comparison = aValue < bValue ? -1 : aValue > bValue ? 1 : 0;
          return sortOrder === 'asc' ? comparison : -comparison;
        });

        return filtered;
      },

      getExecutionById: (executionId) => {
        const { executions } = get();
        return executions.find((exec) => exec.execution_id === executionId) || null;
      },

      getExecutionsByWorkflow: (workflowId) => {
        const { executions } = get();
        return executions.filter((exec) => exec.workflow_id === workflowId);
      },

      getExecutionStats: () => {
        const { executions } = get();

        const stats = {
          total: executions.length,
          completed: 0,
          failed: 0,
          running: 0,
          pending: 0,
        };

        executions.forEach((exec) => {
          switch (exec.status) {
            case 'Completed':
              stats.completed++;
              break;
            case 'Failed':
              stats.failed++;
              break;
            case 'Running':
              stats.running++;
              break;
            case 'Pending':
              stats.pending++;
              break;
          }
        });

        return stats;
      },
    }),
    {
      name: 'execution-store',
    }
  )
);
