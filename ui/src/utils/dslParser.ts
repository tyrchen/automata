import { FlowNode, FlowEdge, WorkflowNode, WorkflowConnection } from '../types';
import { v4 as uuidv4 } from 'uuid';

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
  const inDegree: Record<string, number> = {};

  nodes.forEach(node => {
    adjacency[node] = [];
    inDegree[node] = 0;
  });

  connections.forEach(conn => {
    if (!adjacency[conn.from]) adjacency[conn.from] = [];
    adjacency[conn.from].push(conn.to);
    inDegree[conn.to] = (inDegree[conn.to] || 0) + 1;
  });

  // Topological sort to determine levels
  const queue: string[] = [];
  nodes.forEach(node => {
    if (inDegree[node] === 0 || node === 'trigger') {
      queue.push(node);
      levelMap[node] = 0;
    }
  });

  while (queue.length > 0) {
    const current = queue.shift()!;
    const currentLevel = levelMap[current];

    if (!nodesByLevel[currentLevel]) {
      nodesByLevel[currentLevel] = [];
    }
    nodesByLevel[currentLevel].push(current);

    adjacency[current]?.forEach(neighbor => {
      inDegree[neighbor]--;
      if (inDegree[neighbor] === 0) {
        queue.push(neighbor);
        levelMap[neighbor] = currentLevel + 1;
      }
    });
  }

  // Calculate positions
  const horizontalSpacing = 250;
  const verticalSpacing = 150;
  const startX = 100;
  const startY = 100;

  Object.entries(nodesByLevel).forEach(([level, levelNodes]) => {
    const levelNum = parseInt(level);
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
    // Parse YAML
    const parsed = parseYaml(dslContent);

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

// Simple YAML parser (you might want to use a proper YAML library)
function parseYaml(content: string): ParsedWorkflow | null {
  try {
    // This is a simplified parser. In production, use a proper YAML library
    const lines = content.split('\n');
    const result: any = {
      metadata: {},
      triggers: [],
      nodes: {},
      connections: [],
    };

    let currentSection = '';
    let currentNode = '';
    let currentTrigger = -1;
    let currentConnection = -1;
    let indent = 0;

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) continue;

      const currentIndent = line.length - line.trimStart().length;

      // Top level sections
      if (currentIndent === 0) {
        if (trimmed === 'metadata:') {
          currentSection = 'metadata';
        } else if (trimmed === 'triggers:') {
          currentSection = 'triggers';
        } else if (trimmed === 'nodes:') {
          currentSection = 'nodes';
        } else if (trimmed === 'connections:') {
          currentSection = 'connections';
        }
      } else {
        // Handle different sections
        if (currentSection === 'metadata') {
          const [key, ...valueParts] = trimmed.split(':');
          const value = valueParts.join(':').trim().replace(/^["']|["']$/g, '');
          result.metadata[key.trim()] = value;
        } else if (currentSection === 'triggers') {
          if (trimmed.startsWith('- ')) {
            const triggerType = trimmed.substring(2).replace(':', '');
            result.triggers.push({ [triggerType]: {} });
            currentTrigger = result.triggers.length - 1;
          } else if (currentTrigger >= 0) {
            const [key, ...valueParts] = trimmed.split(':');
            const value = valueParts.join(':').trim().replace(/^["']|["']$/g, '');
            const triggerType = Object.keys(result.triggers[currentTrigger])[0];
            result.triggers[currentTrigger][triggerType][key.trim()] = value;
          }
        } else if (currentSection === 'nodes') {
          if (currentIndent === 2 && trimmed.endsWith(':')) {
            currentNode = trimmed.slice(0, -1);
            result.nodes[currentNode] = {};
          } else if (currentNode) {
            const [key, ...valueParts] = trimmed.split(':');
            const value = valueParts.join(':').trim().replace(/^["']|["']$/g, '');

            if (key.trim() === 'type') {
              result.nodes[currentNode].type = value;
            } else if (key.trim() === 'mapping' || key.trim() === 'rules') {
              result.nodes[currentNode][key.trim()] = {};
            } else {
              result.nodes[currentNode][key.trim()] = value;
            }
          }
        } else if (currentSection === 'connections') {
          if (trimmed.startsWith('- from:')) {
            const from = trimmed.split(':')[1].trim().replace(/^["']|["']$/g, '');
            result.connections.push({ from });
            currentConnection = result.connections.length - 1;
          } else if (currentConnection >= 0) {
            const [key, ...valueParts] = trimmed.split(':');
            const value = valueParts.join(':').trim().replace(/^["']|["']$/g, '');
            result.connections[currentConnection][key.trim()] = value;
          }
        }
      }
    }

    return result as ParsedWorkflow;
  } catch (error) {
    console.error('YAML parse error:', error);
    return null;
  }
}

export function validateWorkflowDsl(dsl: string): { isValid: boolean; errors: string[] } {
  const errors: string[] = [];

  try {
    const parsed = parseYaml(dsl);

    if (!parsed) {
      errors.push('Invalid YAML format');
      return { isValid: false, errors };
    }

    // Validate metadata
    if (!parsed.metadata?.name) {
      errors.push('metadata.name is required');
    }
    if (!parsed.metadata?.version) {
      errors.push('metadata.version is required');
    }

    // Validate triggers
    if (!parsed.triggers || !Array.isArray(parsed.triggers) || parsed.triggers.length === 0) {
      errors.push('At least one trigger is required');
    }

    // Validate nodes
    if (!parsed.nodes || Object.keys(parsed.nodes).length === 0) {
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
  let yaml = '';

  // Metadata
  yaml += 'metadata:\n';
  yaml += `  name: "${workflow.metadata.name}"\n`;
  yaml += `  version: "${workflow.metadata.version}"\n`;
  yaml += `  description: "${workflow.metadata.description}"\n\n`;

  // Triggers
  yaml += 'triggers:\n';
  workflow.triggers.forEach((trigger: any) => {
    const triggerType = Object.keys(trigger)[0];
    yaml += `  - ${triggerType}:`;

    const config = trigger[triggerType];
    if (Object.keys(config).length === 0) {
      yaml += ' {}\n';
    } else {
      yaml += '\n';
      Object.entries(config).forEach(([key, value]) => {
        yaml += `      ${key}: ${JSON.stringify(value)}\n`;
      });
    }
  });
  yaml += '\n';

  // Nodes
  yaml += 'nodes:\n';
  Object.entries(workflow.nodes).forEach(([nodeId, node]: [string, any]) => {
    yaml += `  ${nodeId}:\n`;
    yaml += `    type: ${node.type}\n`;

    Object.entries(node).forEach(([key, value]) => {
      if (key !== 'type') {
        if (typeof value === 'object' && value !== null) {
          yaml += `    ${key}:\n`;
          Object.entries(value).forEach(([subKey, subValue]) => {
            yaml += `      ${subKey}: ${JSON.stringify(subValue)}\n`;
          });
        } else {
          yaml += `    ${key}: ${JSON.stringify(value)}\n`;
        }
      }
    });
    yaml += '\n';
  });

  // Connections
  if (workflow.connections.length > 0) {
    yaml += 'connections:\n';
    workflow.connections.forEach((conn: any) => {
      yaml += `  - from: ${conn.from}\n`;
      yaml += `    to: ${conn.to}\n`;
      if (conn.condition) {
        yaml += `    condition: ${conn.condition}\n`;
      }
    });
  }

  return yaml;
}
