import React, { memo, useCallback } from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import { useWorkflowStore } from '../../../stores';
import { Button } from '../../ui/button';
import { Badge } from '../../ui/badge';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../../ui/tooltip';
import {
  Settings,
  Trash2,
  Play,
  Database,
  Globe,
  Filter,
  RotateCcw,
  Code,
  GitBranch,
  Repeat,
  Zap,
} from 'lucide-react';
import { cn } from '../../../lib/utils';

// Icon mapping for node types
const getNodeIcon = (nodeType: string) => {
  switch (nodeType) {
    case 'http':
      return Globe;
    case 'database':
      return Database;
    case 'transformer':
      return Code;
    case 'validator':
      return Filter;
    case 'conditional':
      return GitBranch;
    case 'forEach':
      return Repeat;
    case 'parallel':
      return Zap;
    default:
      return Play;
  }
};

// Color mapping for node types
const getNodeColor = (nodeType: string) => {
  switch (nodeType) {
    case 'http':
      return 'border-red-400 bg-red-50 text-red-700';
    case 'database':
      return 'border-green-400 bg-green-50 text-green-700';
    case 'transformer':
      return 'border-blue-400 bg-blue-50 text-blue-700';
    case 'validator':
      return 'border-yellow-400 bg-yellow-50 text-yellow-700';
    case 'conditional':
      return 'border-purple-400 bg-purple-50 text-purple-700';
    case 'forEach':
      return 'border-orange-400 bg-orange-50 text-orange-700';
    case 'parallel':
      return 'border-pink-400 bg-pink-50 text-pink-700';
    default:
      return 'border-gray-400 bg-gray-50 text-gray-700';
  }
};

interface WorkflowNodeData {
  label: string;
  nodeType: string;
  config: Record<string, any>;
  description?: string;
  status?: 'idle' | 'running' | 'completed' | 'failed';
  error?: string;
}

const WorkflowNode: React.FC<NodeProps<WorkflowNodeData>> = memo(({
  id,
  data,
  selected,
  dragging,
}) => {
  const {
    removeNode,
    setSelectedNodeId,
    setShowSettings,
  } = useWorkflowStore();

  const Icon = getNodeIcon(data.nodeType);
  const nodeColor = getNodeColor(data.nodeType);

  const handleConfigClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    setSelectedNodeId(id);
    setShowSettings(true);
  }, [id, setSelectedNodeId, setShowSettings]);

  const handleDeleteClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    removeNode(id);
  }, [id, removeNode]);

  const getStatusColor = () => {
    switch (data.status) {
      case 'running':
        return 'bg-blue-500';
      case 'completed':
        return 'bg-green-500';
      case 'failed':
        return 'bg-red-500';
      default:
        return 'bg-gray-400';
    }
  };

  return (
    <TooltipProvider>
      <div
        className={cn(
          'relative min-w-[180px] rounded-lg border-2 bg-background shadow-sm transition-all duration-200',
          nodeColor,
          selected && 'ring-2 ring-primary ring-offset-2',
          dragging && 'opacity-75 scale-105',
          data.status === 'running' && 'animate-pulse',
        )}
      >
        {/* Status indicator */}
        {data.status && data.status !== 'idle' && (
          <div
            className={cn(
              'absolute -top-1 -right-1 w-3 h-3 rounded-full border-2 border-background',
              getStatusColor()
            )}
          />
        )}

        {/* Input handle */}
        <Handle
          type="target"
          position={Position.Left}
          className="w-3 h-3 !bg-primary border-2 border-background"
        />

        {/* Node content */}
        <div className="p-3">
          {/* Header */}
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              <Icon className="w-4 h-4" />
              <span className="font-medium text-sm truncate">
                {data.label}
              </span>
            </div>

            {/* Node type badge */}
            <Badge variant="secondary" className="text-xs">
              {data.nodeType}
            </Badge>
          </div>

          {/* Description */}
          {data.description && (
            <p className="text-xs text-muted-foreground mb-2 line-clamp-2">
              {data.description}
            </p>
          )}

          {/* Configuration summary */}
          <div className="text-xs text-muted-foreground mb-3">
            {Object.keys(data.config).length > 0 ? (
              <div className="space-y-1">
                {Object.entries(data.config).slice(0, 2).map(([key, value]) => (
                  <div key={key} className="flex justify-between">
                    <span className="truncate mr-2">{key}:</span>
                    <span className="font-mono text-xs truncate">
                      {typeof value === 'string' ? value : JSON.stringify(value)}
                    </span>
                  </div>
                ))}
                {Object.keys(data.config).length > 2 && (
                  <div className="text-center">
                    +{Object.keys(data.config).length - 2} more
                  </div>
                )}
              </div>
            ) : (
              <span className="italic">No configuration</span>
            )}
          </div>

          {/* Error message */}
          {data.error && (
            <div className="text-xs text-red-600 bg-red-50 p-1 rounded mb-2">
              {data.error}
            </div>
          )}

          {/* Actions */}
          <div className="flex justify-between items-center">
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleConfigClick}
                  className="h-6 w-6 p-0"
                >
                  <Settings className="w-3 h-3" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>Configure node</p>
              </TooltipContent>
            </Tooltip>

            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleDeleteClick}
                  className="h-6 w-6 p-0 text-red-600 hover:text-red-700 hover:bg-red-50"
                >
                  <Trash2 className="w-3 h-3" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>Delete node</p>
              </TooltipContent>
            </Tooltip>
          </div>
        </div>

        {/* Output handle */}
        <Handle
          type="source"
          position={Position.Right}
          className="w-3 h-3 !bg-primary border-2 border-background"
        />
      </div>
    </TooltipProvider>
  );
});

WorkflowNode.displayName = 'WorkflowNode';

export default WorkflowNode;
