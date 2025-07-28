import React, { useCallback, useRef, useEffect } from 'react';
import {
  ReactFlow,
  MiniMap,
  Controls,
  Background,
  useNodesState,
  useEdgesState,
  addEdge,
  Connection,
  Edge,
  Node,
  ReactFlowProvider,
  Panel,
  useReactFlow,
  BackgroundVariant,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { useWorkflowStore, useAppStore } from '../../stores';
import { FlowNode, FlowEdge, NodeDescription } from '../../types';
import WorkflowNode from './nodes/WorkflowNode';
import { Button } from '../ui/button';
import { ZoomIn, ZoomOut, Maximize, Save, Play } from 'lucide-react';

// Custom node types
const nodeTypes = {
  workflowNode: WorkflowNode,
};

interface ExecutionProgress {
  current_node?: string;
  completed_nodes: string[];
  failed_nodes: string[];
  total_nodes: number;
  progress_percentage: number;
}

interface WorkflowCanvasProps {
  onSave?: () => void;
  onExecute?: () => void;
  readOnly?: boolean;
  executionProgress?: ExecutionProgress | null;
}

const WorkflowCanvasContent: React.FC<WorkflowCanvasProps> = ({
  onSave,
  onExecute,
  readOnly = false,
  executionProgress = null,
}) => {
  const reactFlow = useReactFlow();
  const dragRef = useRef<HTMLDivElement>(null);

  const {
    nodes: storeNodes,
    edges: storeEdges,
    setNodes,
    setEdges,
    addNode,
    addEdge: addStoreEdge,
    removeNode,
    removeEdge,
    setSelectedNodeId,
    selectedNodeId,
    draggedNodeType,
    setDraggedNodeType,
    availableNodes,
    isSaving,
  } = useWorkflowStore();

  const {
    preferences: { showMinimap, snapToGrid, gridSize },
  } = useAppStore();

  const [nodes, , onNodesChange] = useNodesState(storeNodes);
  const [edges, , onEdgesChange] = useEdgesState(storeEdges);

  // Sync store state with React Flow
  useEffect(() => {
    if (JSON.stringify(nodes) !== JSON.stringify(storeNodes)) {
      setNodes(storeNodes);
    }
  }, [storeNodes, setNodes]);

  useEffect(() => {
    if (JSON.stringify(edges) !== JSON.stringify(storeEdges)) {
      setEdges(storeEdges);
    }
  }, [storeEdges, setEdges]);

  // Update node styling based on execution progress
  useEffect(() => {
    if (executionProgress && storeNodes.length > 0) {
      const updatedNodes = storeNodes.map(node => {
        let nodeStyle = { ...(node as any).style };

        if (executionProgress.completed_nodes.includes(node.id)) {
          // Completed nodes - green border
          nodeStyle = {
            ...nodeStyle,
            border: '2px solid #22c55e',
            backgroundColor: '#dcfce7',
          };
        } else if (executionProgress.failed_nodes.includes(node.id)) {
          // Failed nodes - red border
          nodeStyle = {
            ...nodeStyle,
            border: '2px solid #ef4444',
            backgroundColor: '#fef2f2',
          };
        } else if (executionProgress.current_node === node.id) {
          // Current node - blue border with pulse animation
          nodeStyle = {
            ...nodeStyle,
            border: '2px solid #3b82f6',
            backgroundColor: '#dbeafe',
            animation: 'pulse 2s infinite',
          };
        } else {
          // Pending nodes - default styling
          nodeStyle = {
            ...nodeStyle,
            border: '1px solid #e5e7eb',
            backgroundColor: '#ffffff',
          };
        }

        return {
          ...node,
          style: nodeStyle,
        } as any;
      });

      setNodes(updatedNodes);
    }
  }, [executionProgress, storeNodes, setNodes]);

  const onConnect = useCallback(
    (params: Connection) => {
      if (params.source && params.target) {
        const newEdge: Omit<FlowEdge, 'id'> = {
          source: params.source,
          target: params.target,
          from: params.source,
          to: params.target,
          type: 'default',
        };
        addStoreEdge(newEdge);
      }
    },
    [addStoreEdge]
  );

  const onNodeClick = useCallback(
    (event: React.MouseEvent, node: Node) => {
      setSelectedNodeId(node.id);
    },
    [setSelectedNodeId]
  );

  const onPaneClick = useCallback(() => {
    setSelectedNodeId(null);
  }, [setSelectedNodeId]);

  const onNodeDelete = useCallback(
    (nodeId: string) => {
      removeNode(nodeId);
    },
    [removeNode]
  );

  const onEdgeDelete = useCallback(
    (edgeId: string) => {
      removeEdge(edgeId);
    },
    [removeEdge]
  );

  // Handle drag and drop from node library
  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();

      if (!draggedNodeType) return;

      const reactFlowBounds = dragRef.current?.getBoundingClientRect();
      if (!reactFlowBounds) return;

      const position = reactFlow.screenToFlowPosition({
        x: event.clientX - reactFlowBounds.left,
        y: event.clientY - reactFlowBounds.top,
      });

      const nodeDescription = availableNodes.find(
        (node) => node.node_type === draggedNodeType
      );

      const newNode: Omit<FlowNode, 'id'> = {
        node_type: draggedNodeType,
        config: {},
        position: {
          x: snapToGrid ? Math.round(position.x / gridSize) * gridSize : position.x,
          y: snapToGrid ? Math.round(position.y / gridSize) * gridSize : position.y,
        },
        data: {
          label: nodeDescription?.node_type || draggedNodeType,
          nodeType: draggedNodeType,
          config: {},
          description: nodeDescription?.description,
        },
        type: 'workflowNode',
      };

      addNode(newNode);
      setDraggedNodeType(null);
    },
    [
      draggedNodeType,
      reactFlow,
      snapToGrid,
      gridSize,
      availableNodes,
      addNode,
      setDraggedNodeType,
    ]
  );

  const handleZoomIn = useCallback(() => {
    reactFlow.zoomIn();
  }, [reactFlow]);

  const handleZoomOut = useCallback(() => {
    reactFlow.zoomOut();
  }, [reactFlow]);

  const handleFitView = useCallback(() => {
    reactFlow.fitView();
  }, [reactFlow]);

  const handleSave = useCallback(() => {
    onSave?.();
  }, [onSave]);

  const handleExecute = useCallback(() => {
    onExecute?.();
  }, [onExecute]);

  return (
    <div className="w-full h-full relative" ref={dragRef}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeClick={onNodeClick}
        onPaneClick={onPaneClick}
        onDragOver={onDragOver}
        onDrop={onDrop}
        nodeTypes={nodeTypes}
        fitView
        snapToGrid={snapToGrid}
        snapGrid={[gridSize, gridSize]}
        defaultViewport={{ x: 0, y: 0, zoom: 1 }}
        minZoom={0.1}
        maxZoom={2}
        deleteKeyCode={readOnly ? null : ['Backspace', 'Delete']}
        multiSelectionKeyCode={readOnly ? null : ['Meta', 'Ctrl']}
        panOnDrag={!readOnly}
        nodesConnectable={!readOnly}
        nodesDraggable={!readOnly}
        elementsSelectable={!readOnly}
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={gridSize}
          size={1}
        />

        <Controls
          showZoom={true}
          showFitView={true}
          showInteractive={true}
        />

        {showMinimap && (
          <MiniMap
            style={{
              height: 120,
              width: 200,
            }}
            zoomable
            pannable
            nodeStrokeColor={(node) => {
              if (node.id === selectedNodeId) return '#ff0073';
              return '#ccc';
            }}
            nodeColor={(node) => {
              return node.data?.nodeType === 'http' ? '#ff6b6b' :
                     node.data?.nodeType === 'database' ? '#4ecdc4' :
                     node.data?.nodeType === 'transformer' ? '#45b7d1' :
                     node.data?.nodeType === 'validator' ? '#f9ca24' :
                     '#6c5ce7';
            }}
          />
        )}

        <Panel position="top-right" className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={handleZoomIn}
            className="bg-background"
          >
            <ZoomIn className="h-4 w-4" />
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={handleZoomOut}
            className="bg-background"
          >
            <ZoomOut className="h-4 w-4" />
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={handleFitView}
            className="bg-background"
          >
            <Maximize className="h-4 w-4" />
          </Button>
          {!readOnly && (
            <>
              <Button
                variant="outline"
                size="sm"
                onClick={handleSave}
                disabled={isSaving}
                className="bg-background"
              >
                <Save className="h-4 w-4 mr-1" />
                {isSaving ? 'Saving...' : 'Save'}
              </Button>
              <Button
                variant="default"
                size="sm"
                onClick={handleExecute}
                className="bg-primary"
              >
                <Play className="h-4 w-4 mr-1" />
                Execute
              </Button>
            </>
          )}
        </Panel>

        {/* Node count indicator */}
        <Panel position="bottom-left" className="bg-background p-2 rounded border">
          <span className="text-xs text-muted-foreground">
            {nodes.length} nodes, {edges.length} connections
          </span>
        </Panel>
      </ReactFlow>
    </div>
  );
};

const WorkflowCanvas: React.FC<WorkflowCanvasProps> = (props) => {
  return (
    <ReactFlowProvider>
      <WorkflowCanvasContent {...props} />
    </ReactFlowProvider>
  );
};

export default WorkflowCanvas;
