import { FlowNode, FlowEdge, WorkflowNode, WorkflowConnection } from '../types';
import { v4 as uuidv4 } from 'uuid';
import yaml from 'js-yaml';

interface ParsedWorkflow {
  metadata: {
    name: string;
    version: string;
    description?: string;
  };
  triggers: any[];
  nodes: Record<string, any>;
  connections: Array<{
    from: string;
    to: string;
    condition?: string;
  }>;
}

interface ParseResult {
  isValid: boolean;
  error?: string;
  nodes?: FlowNode[];
  edges?: FlowEdge[];
  workflow?: ParsedWorkflow;
}

// Node type colors and icons
const NODE_STYLES = {
  trigger: { color: '#4ade80', icon: '⚡' },
  transformer: { color: '#3b82f6', icon: '🔄' },
  validator: { color: '#f59e0b', icon: '✓' },
  http: { color: '#ec4899', icon: '🌐' },
  database: { color: '#8b5cf6', icon: '💾' },
  conditional: { color: '#10b981', icon: '❓' },
  forEach: { color: '#6366f1', icon: '🔁' },
  parallel: { color: '#14b8a6', icon: '⚡' },
  email: { color: '#f43f5e', icon: '✉️' },
  default: { color: '#6b7280', icon: '📦' },
};

// Layout algorithm for nodes
function calculateNodePositions(
  nodes: string[],
  connections: WorkflowConnection[]
): Record<string, { x: number; y: number }> {
  const positions: Record<string, { x: number; y: number }> = {};
  const levelMap: Record<string, number> = {};
  const nodesByLevel: Record<number, string[]> = {};

  // Build adjacency list
  const adjacency: Record<string, string[]> = {};
  nodes.forEach(node => {
    adjacency[node] = [];
  });

  connections.forEach(conn => {
    if (adjacency[conn.from]) {
      adjacency[conn.from].push(conn.to);
    }
  });

  // Determine levels using BFS
  const visited = new Set<string>();
  const queue = ['trigger'];
  levelMap['trigger'] = 0;

  while (queue.length > 0) {
    const current = queue.shift()!;
    if (visited.has(current)) continue;
    visited.add(current);

    const level = levelMap[current];
    if (!nodesByLevel[level]) {
      nodesByLevel[level] = [];
    }
    nodesByLevel[level].push(current);

    if (adjacency[current]) {
      adjacency[current].forEach(neighbor => {
        if (!visited.has(neighbor)) {
          levelMap[neighbor] = level + 1;
          queue.push(neighbor);
        }
      });
    }
  }

  // Position nodes
  const horizontalSpacing = 300;
  const verticalSpacing = 100;
  const startX = 100;
  const startY = 100;

  Object.entries(nodesByLevel).forEach(([levelStr, levelNodes]) => {
    const levelNum = parseInt(levelStr);
    const nodesCount = levelNodes.length;

    levelNodes.forEach((node, index) => {
      const x = startX + levelNum * horizontalSpacing;
      const y = startY + (index - (nodesCount - 1) / 2) * verticalSpacing;
      positions[node] = { x, y };
    });
  });

  // Handle disconnected nodes
  nodes.forEach(node => {
    if (!positions[node]) {
      positions[node] = {
        x: startX + Object.keys(nodesByLevel).length * horizontalSpacing,
        y: startY + Object.keys(positions).length * 50,
      };
    }
  });

  return positions;
}

export function parseDsl(dslContent: string): ParseResult {
  try {
    // Parse YAML using js-yaml
    const parsed = yaml.load(dslContent) as ParsedWorkflow;

    if (!parsed) {
      return { isValid: false, error: 'Invalid YAML format' };
    }

    // Validate required fields
    if (!parsed.metadata?.name || !parsed.metadata?.version) {
      return { isValid: false, error: 'Missing required metadata fields (name, version)' };
    }

    if (!parsed.triggers || !Array.isArray(parsed.triggers) || parsed.triggers.length === 0) {
      return { isValid: false, error: 'At least one trigger is required' };
    }

    if (!parsed.nodes || typeof parsed.nodes !== 'object') {
      return { isValid: false, error: 'Nodes section is required' };
    }

    const nodes: FlowNode[] = [];
    const edges: FlowEdge[] = [];
    const nodeIds = ['trigger', ...Object.keys(parsed.nodes)];

    // Create trigger node
    const triggerNode: FlowNode = {
      id: 'trigger',
      type: 'workflowNode',
      position: { x: 0, y: 0 },
      data: {
        label: 'Trigger',
        nodeType: 'trigger',
        config: parsed.triggers[0],
        description: `Trigger: ${Object.keys(parsed.triggers[0])[0]}`,
      },
      node_type: 'trigger',
      config: parsed.triggers[0],
    };
    nodes.push(triggerNode);

    // Create workflow nodes
    Object.entries(parsed.nodes).forEach(([nodeId, nodeConfig]: [string, any]) => {
      const nodeType = nodeConfig.type || 'default';
      const style = NODE_STYLES[nodeType as keyof typeof NODE_STYLES] || NODE_STYLES.default;

      const node: FlowNode = {
        id: nodeId,
        type: 'workflowNode',
        position: { x: 0, y: 0 },
        data: {
          label: `${style.icon} ${nodeId}`,
          nodeType: nodeType,
          config: nodeConfig,
          description: nodeConfig.description || `${nodeType} node`,
        },
        node_type: nodeType,
        config: nodeConfig,
      };

      nodes.push(node);
    });

    // Create connections
    if (parsed.connections && Array.isArray(parsed.connections)) {
      parsed.connections.forEach((conn: any, index: number) => {
        if (conn.from && conn.to) {
          const edge: FlowEdge = {
            id: `edge-${index}-${uuidv4()}`,
            source: conn.from,
            target: conn.to,
            from: conn.from,
            to: conn.to,
            type: 'default',
            animated: true,
            label: conn.condition ? `if: ${conn.condition}` : undefined,
            condition: conn.condition,
          };
          edges.push(edge);
        }
      });
    }

    // Calculate node positions
    const positions = calculateNodePositions(
      nodeIds,
      parsed.connections || []
    );

    // Update node positions
    nodes.forEach(node => {
      const pos = positions[node.id];
      if (pos) {
        node.position = pos;
      }
    });

    return {
      isValid: true,
      nodes,
      edges,
      workflow: parsed,
    };
  } catch (error) {
    return {
      isValid: false,
      error: error instanceof Error ? error.message : 'Failed to parse DSL',
    };
  }
}

export function validateWorkflowDsl(dsl: string): { isValid: boolean; errors: string[] } {
  const errors: string[] = [];

  try {
    const parsed = yaml.load(dsl) as ParsedWorkflow;
    if (!parsed) {
      errors.push('Invalid YAML format');
      return { isValid: false, errors };
    }

    // Validate metadata
    if (!parsed.metadata?.name) {
      errors.push('Metadata must include a name');
    }
    if (!parsed.metadata?.version) {
      errors.push('Metadata must include a version');
    }

    // Validate triggers
    if (!parsed.triggers || !Array.isArray(parsed.triggers) || parsed.triggers.length === 0) {
      errors.push('At least one trigger is required');
    }

    // Validate nodes
    if (!parsed.nodes || typeof parsed.nodes !== 'object' || Object.keys(parsed.nodes).length === 0) {
      errors.push('At least one node is required');
    }

    // Validate node types
    const validNodeTypes = [
      'transformer', 'validator', 'http', 'database',
      'conditional', 'forEach', 'parallel', 'email'
    ];

    Object.entries(parsed.nodes || {}).forEach(([nodeId, node]: [string, any]) => {
      if (!node.type) {
        errors.push(`Node '${nodeId}' must have a type`);
      } else if (!validNodeTypes.includes(node.type)) {
        errors.push(`Node '${nodeId}' has invalid type '${node.type}'`);
      }
    });

    // Validate connections
    if (parsed.connections && Array.isArray(parsed.connections)) {
      const nodeIds = ['trigger', ...Object.keys(parsed.nodes || {})];

      parsed.connections.forEach((conn, index) => {
        if (!conn.from) {
          errors.push(`Connection ${index + 1} must have 'from' field`);
        } else if (!nodeIds.includes(conn.from)) {
          errors.push(`Connection ${index + 1}: unknown source node '${conn.from}'`);
        }

        if (!conn.to) {
          errors.push(`Connection ${index + 1} must have 'to' field`);
        } else if (!nodeIds.includes(conn.to)) {
          errors.push(`Connection ${index + 1}: unknown target node '${conn.to}'`);
        }
      });
    }

    return { isValid: errors.length === 0, errors };
  } catch (error) {
    errors.push(error instanceof Error ? error.message : 'Validation failed');
    return { isValid: false, errors };
  }
}

export function generateWorkflowDsl(nodes: FlowNode[], edges: FlowEdge[]): string {
  const workflow: any = {
    metadata: {
      name: 'Generated Workflow',
      version: '1.0.0',
      description: 'Workflow generated from visual editor',
    },
    triggers: [],
    nodes: {},
    connections: [],
  };

  // Find trigger node
  const triggerNode = nodes.find(n => n.node_type === 'trigger');
  if (triggerNode) {
    workflow.triggers.push(triggerNode.config || { manual: {} });
  } else {
    workflow.triggers.push({ manual: {} });
  }

  // Add nodes (excluding trigger)
  nodes.forEach(node => {
    if (node.node_type !== 'trigger') {
      workflow.nodes[node.id] = {
        type: node.node_type,
        ...node.config,
      };
    }
  });

  // Add connections
  edges.forEach(edge => {
    const connection: any = {
      from: edge.source,
      to: edge.target,
    };

    if (edge.condition) {
      connection.condition = edge.condition;
    }

    workflow.connections.push(connection);
  });

  // Convert to YAML-like string
  let yamlOutput = '';

  // Metadata
  yamlOutput += 'metadata:\n';
  yamlOutput += `  name: "${workflow.metadata.name}"\n`;
  yamlOutput += `  version: "${workflow.metadata.version}"\n`;
  yamlOutput += `  description: "${workflow.metadata.description}"\n\n`;

  // Triggers
  yamlOutput += 'triggers:\n';
  workflow.triggers.forEach((trigger: any) => {
    const triggerType = Object.keys(trigger)[0];
    yamlOutput += `  - ${triggerType}: {}\n`;
  });
  yamlOutput += '\n';

  // Nodes
  yamlOutput += 'nodes:\n';
  Object.entries(workflow.nodes).forEach(([nodeId, nodeConfig]: [string, any]) => {
    yamlOutput += `  ${nodeId}:\n`;
    yamlOutput += `    type: ${nodeConfig.type}\n`;

    // Add other node properties
    Object.entries(nodeConfig).forEach(([key, value]) => {
      if (key !== 'type') {
        yamlOutput += `    ${key}: ${JSON.stringify(value)}\n`;
      }
    });
  });
  yamlOutput += '\n';

  // Connections
  yamlOutput += 'connections:\n';
  workflow.connections.forEach((conn: any) => {
    yamlOutput += `  - from: ${conn.from}\n`;
    yamlOutput += `    to: ${conn.to}\n`;
    if (conn.condition) {
      yamlOutput += `    condition: ${conn.condition}\n`;
    }
  });

  return yamlOutput;
}
