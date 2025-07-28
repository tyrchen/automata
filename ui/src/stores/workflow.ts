import { create } from 'zustand';
import { devtools } from 'zustand/middleware';
import {
  WorkflowDefinition,
  WorkflowListItem,
  FlowNode,
  FlowEdge,
  NodeDescription,
  CreateWorkflowRequest,
  WorkflowFormData,
  ApiError,
} from '../types';

interface WorkflowState {
  // Current workflow being edited
  currentWorkflow: WorkflowDefinition | null;

  // List of all workflows
  workflows: WorkflowListItem[];

  // Available node types and descriptions
  availableNodes: NodeDescription[];

  // Current workflow canvas state
  nodes: FlowNode[];
  edges: FlowEdge[];

  // UI state
  isLoading: boolean;
  isSaving: boolean;
  selectedNodeId: string | null;
  showNodeLibrary: boolean;
  showSettings: boolean;
  error: ApiError | null;

  // Canvas interaction state
  draggedNodeType: string | null;
  canvasViewport: { x: number; y: number; zoom: number };

  // Actions
  setCurrentWorkflow: (workflow: WorkflowDefinition | null) => void;
  setWorkflows: (workflows: WorkflowListItem[]) => void;
  setAvailableNodes: (nodes: NodeDescription[]) => void;
  setNodes: (nodes: FlowNode[]) => void;
  setEdges: (edges: FlowEdge[]) => void;
  addNode: (node: Omit<FlowNode, 'id'>) => void;
  updateNode: (nodeId: string, updates: Partial<FlowNode>) => void;
  removeNode: (nodeId: string) => void;
  addEdge: (edge: Omit<FlowEdge, 'id'>) => void;
  updateEdge: (edgeId: string, updates: Partial<FlowEdge>) => void;
  removeEdge: (edgeId: string) => void;
  setSelectedNodeId: (nodeId: string | null) => void;
  setShowNodeLibrary: (show: boolean) => void;
  setShowSettings: (show: boolean) => void;
  setError: (error: ApiError | null) => void;
  setIsLoading: (loading: boolean) => void;
  setIsSaving: (saving: boolean) => void;
  setDraggedNodeType: (nodeType: string | null) => void;
  setCanvasViewport: (viewport: { x: number; y: number; zoom: number }) => void;
  clearWorkflow: () => void;

  // Computed getters
  getSelectedNode: () => FlowNode | null;
  getNodeById: (nodeId: string) => FlowNode | null;
  getConnectedNodes: (nodeId: string) => { incoming: FlowNode[]; outgoing: FlowNode[] };
}

export const useWorkflowStore = create<WorkflowState>()(
  devtools(
    (set, get) => ({
      // Initial state
      currentWorkflow: null,
      workflows: [],
      availableNodes: [],
      nodes: [],
      edges: [],
      isLoading: false,
      isSaving: false,
      selectedNodeId: null,
      showNodeLibrary: true,
      showSettings: false,
      error: null,
      draggedNodeType: null,
      canvasViewport: { x: 0, y: 0, zoom: 1 },

      // Actions
      setCurrentWorkflow: (workflow) => set({ currentWorkflow: workflow }),

      setWorkflows: (workflows) => set({ workflows }),

      setAvailableNodes: (nodes) => set({ availableNodes: nodes }),

      setNodes: (nodes) => set({ nodes }),

      setEdges: (edges) => set({ edges }),

      addNode: (nodeData) => {
        const id = `node_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        const node: FlowNode = {
          ...nodeData,
          id,
        };
        set((state) => ({
          nodes: [...state.nodes, node],
        }));
      },

      updateNode: (nodeId, updates) =>
        set((state) => ({
          nodes: state.nodes.map((node) =>
            node.id === nodeId ? { ...node, ...updates } : node
          ),
        })),

      removeNode: (nodeId) =>
        set((state) => ({
          nodes: state.nodes.filter((node) => node.id !== nodeId),
          edges: state.edges.filter(
            (edge) => edge.source !== nodeId && edge.target !== nodeId
          ),
          selectedNodeId:
            state.selectedNodeId === nodeId ? null : state.selectedNodeId,
        })),

      addEdge: (edgeData) => {
        const id = `edge_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        const edge: FlowEdge = {
          ...edgeData,
          id,
          from: edgeData.source,
          to: edgeData.target,
        };
        set((state) => ({
          edges: [...state.edges, edge],
        }));
      },

      updateEdge: (edgeId, updates) =>
        set((state) => ({
          edges: state.edges.map((edge) =>
            edge.id === edgeId ? { ...edge, ...updates } : edge
          ),
        })),

      removeEdge: (edgeId) =>
        set((state) => ({
          edges: state.edges.filter((edge) => edge.id !== edgeId),
        })),

      setSelectedNodeId: (nodeId) => set({ selectedNodeId: nodeId }),

      setShowNodeLibrary: (show) => set({ showNodeLibrary: show }),

      setShowSettings: (show) => set({ showSettings: show }),

      setError: (error) => set({ error }),

      setIsLoading: (loading) => set({ isLoading: loading }),

      setIsSaving: (saving) => set({ isSaving: saving }),

      setDraggedNodeType: (nodeType) => set({ draggedNodeType: nodeType }),

      setCanvasViewport: (viewport) => set({ canvasViewport: viewport }),

      clearWorkflow: () =>
        set({
          currentWorkflow: null,
          nodes: [],
          edges: [],
          selectedNodeId: null,
          error: null,
        }),

      // Computed getters
      getSelectedNode: () => {
        const { nodes, selectedNodeId } = get();
        return selectedNodeId
          ? nodes.find((node) => node.id === selectedNodeId) || null
          : null;
      },

      getNodeById: (nodeId) => {
        const { nodes } = get();
        return nodes.find((node) => node.id === nodeId) || null;
      },

      getConnectedNodes: (nodeId) => {
        const { nodes, edges } = get();
        const incoming = edges
          .filter((edge) => edge.target === nodeId)
          .map((edge) => nodes.find((node) => node.id === edge.source))
          .filter(Boolean) as FlowNode[];

        const outgoing = edges
          .filter((edge) => edge.source === nodeId)
          .map((edge) => nodes.find((node) => node.id === edge.target))
          .filter(Boolean) as FlowNode[];

        return { incoming, outgoing };
      },
    }),
    {
      name: 'workflow-store',
    }
  )
);
