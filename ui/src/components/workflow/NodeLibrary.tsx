import React, { useState, useMemo } from 'react';
import { useWorkflowStore } from '../../stores';
import { NodeDescription, NodeCategory } from '../../types';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Badge } from '../ui/badge';
import { ScrollArea } from '../ui/scroll-area';
import { Separator } from '../ui/separator';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../ui/tooltip';
import {
  Search,
  X,
  ChevronDown,
  ChevronRight,
  Database,
  Globe,
  Filter,
  RotateCcw,
  Code,
  GitBranch,
  Repeat,
  Zap,
  FileText,
  HardDrive,
  Puzzle,
} from 'lucide-react';
import { cn } from '../../lib/utils';

// Icon mapping for node categories
const getCategoryIcon = (category: NodeCategory) => {
  switch (category) {
    case NodeCategory.Http:
      return Globe;
    case NodeCategory.Database:
      return Database;
    case NodeCategory.Transform:
      return Code;
    case NodeCategory.Validation:
      return Filter;
    case NodeCategory.Control:
      return GitBranch;
    case NodeCategory.Communication:
      return Zap;
    case NodeCategory.Storage:
      return HardDrive;
    case NodeCategory.Custom:
      return Puzzle;
    default:
      return FileText;
  }
};

// Icon mapping for specific node types
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
      return FileText;
  }
};

// Categorize nodes
const categorizeNodes = (nodes: NodeDescription[]): Record<NodeCategory, NodeDescription[]> => {
  const categories: Record<NodeCategory, NodeDescription[]> = {
    [NodeCategory.Http]: [],
    [NodeCategory.Database]: [],
    [NodeCategory.Transform]: [],
    [NodeCategory.Validation]: [],
    [NodeCategory.Control]: [],
    [NodeCategory.Communication]: [],
    [NodeCategory.Storage]: [],
    [NodeCategory.Custom]: [],
  };

  nodes.forEach((node) => {
    const nodeType = node.node_type.toLowerCase();

    if (nodeType.includes('http') || nodeType.includes('api') || nodeType.includes('webhook')) {
      categories[NodeCategory.Http].push(node);
    } else if (nodeType.includes('database') || nodeType.includes('query') || nodeType.includes('sql')) {
      categories[NodeCategory.Database].push(node);
    } else if (nodeType.includes('transform') || nodeType.includes('map') || nodeType.includes('convert')) {
      categories[NodeCategory.Transform].push(node);
    } else if (nodeType.includes('validate') || nodeType.includes('check') || nodeType.includes('verify')) {
      categories[NodeCategory.Validation].push(node);
    } else if (nodeType.includes('condition') || nodeType.includes('if') || nodeType.includes('switch') ||
               nodeType.includes('parallel') || nodeType.includes('foreach') || nodeType.includes('loop')) {
      categories[NodeCategory.Control].push(node);
    } else if (nodeType.includes('email') || nodeType.includes('slack') || nodeType.includes('notification')) {
      categories[NodeCategory.Communication].push(node);
    } else if (nodeType.includes('file') || nodeType.includes('storage') || nodeType.includes('s3')) {
      categories[NodeCategory.Storage].push(node);
    } else {
      categories[NodeCategory.Custom].push(node);
    }
  });

  return categories;
};

interface NodeLibraryProps {
  onClose?: () => void;
}

const NodeLibrary: React.FC<NodeLibraryProps> = ({ onClose }) => {
  const { availableNodes, showNodeLibrary, setShowNodeLibrary, setDraggedNodeType } = useWorkflowStore();
  const [searchTerm, setSearchTerm] = useState('');
  const [expandedCategories, setExpandedCategories] = useState<Set<NodeCategory>>(
    new Set([NodeCategory.Http, NodeCategory.Database, NodeCategory.Transform])
  );

  const categorizedNodes = useMemo(() => categorizeNodes(availableNodes), [availableNodes]);

  const filteredNodes = useMemo(() => {
    if (!searchTerm) return categorizedNodes;

    const filtered: Record<NodeCategory, NodeDescription[]> = {
      [NodeCategory.Http]: [],
      [NodeCategory.Database]: [],
      [NodeCategory.Transform]: [],
      [NodeCategory.Validation]: [],
      [NodeCategory.Control]: [],
      [NodeCategory.Communication]: [],
      [NodeCategory.Storage]: [],
      [NodeCategory.Custom]: [],
    };

    Object.entries(categorizedNodes).forEach(([category, nodes]) => {
      filtered[category as NodeCategory] = nodes.filter(
        (node) =>
          node.node_type.toLowerCase().includes(searchTerm.toLowerCase()) ||
          node.description.toLowerCase().includes(searchTerm.toLowerCase())
      );
    });

    return filtered;
  }, [categorizedNodes, searchTerm]);

  const toggleCategory = (category: NodeCategory) => {
    const newExpanded = new Set(expandedCategories);
    if (newExpanded.has(category)) {
      newExpanded.delete(category);
    } else {
      newExpanded.add(category);
    }
    setExpandedCategories(newExpanded);
  };

  const handleDragStart = (event: React.DragEvent, nodeType: string) => {
    setDraggedNodeType(nodeType);
    event.dataTransfer.effectAllowed = 'move';
    event.dataTransfer.setData('application/reactflow', nodeType);
  };

  const handleDragEnd = () => {
    setDraggedNodeType(null);
  };

  const handleClose = () => {
    setShowNodeLibrary(false);
    onClose?.();
  };

  if (!showNodeLibrary) return null;

  return (
    <TooltipProvider>
      <div className="w-80 h-full bg-background border-r border-border flex flex-col">
        {/* Header */}
        <div className="p-4 border-b border-border">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-lg font-semibold">Node Library</h2>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleClose}
              className="h-8 w-8 p-0"
            >
              <X className="h-4 w-4" />
            </Button>
          </div>

          {/* Search */}
          <div className="relative">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search nodes..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="pl-10"
            />
          </div>
        </div>

        {/* Node Categories */}
        <ScrollArea className="flex-1 p-4">
          <div className="space-y-4">
            {Object.entries(filteredNodes).map(([category, nodes]) => {
              if (nodes.length === 0) return null;

              const CategoryIcon = getCategoryIcon(category as NodeCategory);
              const isExpanded = expandedCategories.has(category as NodeCategory);

              return (
                <div key={category} className="space-y-2">
                  {/* Category Header */}
                  <Button
                    variant="ghost"
                    className="w-full justify-between p-2 h-auto"
                    onClick={() => toggleCategory(category as NodeCategory)}
                  >
                    <div className="flex items-center gap-2">
                      <CategoryIcon className="h-4 w-4" />
                      <span className="font-medium">{category}</span>
                      <Badge variant="secondary" className="ml-auto">
                        {nodes.length}
                      </Badge>
                    </div>
                    {isExpanded ? (
                      <ChevronDown className="h-4 w-4" />
                    ) : (
                      <ChevronRight className="h-4 w-4" />
                    )}
                  </Button>

                  {/* Category Nodes */}
                  {isExpanded && (
                    <div className="space-y-1 ml-4">
                      {nodes.map((node) => {
                        const NodeIcon = getNodeIcon(node.node_type);

                        return (
                          <Tooltip key={node.node_type}>
                            <TooltipTrigger asChild>
                              <div
                                draggable
                                onDragStart={(e) => handleDragStart(e, node.node_type)}
                                onDragEnd={handleDragEnd}
                                className={cn(
                                  'flex items-center gap-3 p-3 rounded-lg border border-border bg-card cursor-grab active:cursor-grabbing',
                                  'hover:bg-accent hover:text-accent-foreground transition-colors',
                                  'active:scale-95 transform transition-transform'
                                )}
                              >
                                <NodeIcon className="h-5 w-5 text-muted-foreground" />
                                <div className="flex-1 min-w-0">
                                  <div className="font-medium text-sm truncate">
                                    {node.node_type}
                                  </div>
                                  <div className="text-xs text-muted-foreground truncate">
                                    {node.description}
                                  </div>
                                </div>
                              </div>
                            </TooltipTrigger>
                            <TooltipContent side="right" className="max-w-xs">
                              <div className="space-y-2">
                                <div className="font-medium">{node.node_type}</div>
                                <div className="text-sm">{node.description}</div>

                                {/* Input/Output info */}
                                <div className="text-xs space-y-1">
                                  {node.inputs.required.length > 0 && (
                                    <div>
                                      <span className="font-medium">Required inputs:</span>{' '}
                                      {node.inputs.required.join(', ')}
                                    </div>
                                  )}

                                  {node.config.required.length > 0 && (
                                    <div>
                                      <span className="font-medium">Required config:</span>{' '}
                                      {node.config.required.join(', ')}
                                    </div>
                                  )}
                                </div>

                                <div className="text-xs text-muted-foreground">
                                  Drag to canvas to add
                                </div>
                              </div>
                            </TooltipContent>
                          </Tooltip>
                        );
                      })}
                    </div>
                  )}

                  {category !== NodeCategory.Custom && <Separator />}
                </div>
              );
            })}

            {/* No results */}
            {searchTerm && Object.values(filteredNodes).every(nodes => nodes.length === 0) && (
              <div className="text-center py-8 text-muted-foreground">
                <FileText className="h-12 w-12 mx-auto mb-4 opacity-50" />
                <p className="text-sm">No nodes found matching "{searchTerm}"</p>
              </div>
            )}
          </div>
        </ScrollArea>

        {/* Footer */}
        <div className="p-4 border-t border-border">
          <div className="text-xs text-muted-foreground text-center">
            Drag nodes to canvas to build your workflow
          </div>
        </div>
      </div>
    </TooltipProvider>
  );
};

export default NodeLibrary;
