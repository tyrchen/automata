/**
 * TypeScript types for Automata workflow engine
 * Based on Rust API models from src/api/models.rs and src/core/workflow.rs
 */

export interface WorkflowMetadata {
  name: string;
  version: string;
  description?: string;
  tags: string[];
  author?: string;
  organization?: string;
}

export interface NodePosition {
  x: number;
  y: number;
}

export interface RetryConfig {
  max_attempts: number;
  delay_ms: number;
  backoff_multiplier?: number;
  max_delay_ms?: number;
}

export interface WorkflowNode {
  id: string;
  node_type: string;
  config: Record<string, any>;
  position?: NodePosition;
  condition?: string;
  timeout?: number;
  retry?: RetryConfig;
}

export interface WorkflowConnection {
  from: string;
  to: string;
  condition?: string;
  label?: string;
}

export interface WorkflowTest {
  name: string;
  input: Record<string, any>;
  mocks?: Record<string, any>;
  expected: Record<string, any>;
}

export type WorkflowTrigger =
  | {
      type: 'webhook';
      path: string;
      method: string;
      auth?: string;
      headers?: Record<string, string>;
    }
  | {
      type: 'schedule';
      cron: string;
      timezone?: string;
    }
  | {
      type: 'event';
      source: string;
      event_type: string;
      filters?: Record<string, any>;
    }
  | {
      type: 'manual';
      parameters?: Record<string, any>;
    };

export interface WorkflowDefinition {
  id: string;
  metadata: WorkflowMetadata;
  triggers: WorkflowTrigger[];
  nodes: Record<string, WorkflowNode>;
  connections: WorkflowConnection[];
  tests?: WorkflowTest[];
  created_at: string;
  updated_at: string;
}

// API Request/Response types
export interface CreateWorkflowRequest {
  name: string;
  description?: string;
  definition: string; // YAML workflow definition
}

export interface CreateWorkflowResponse {
  id: string;
  name: string;
  status: string;
}

export interface GetWorkflowResponse {
  id: string;
  metadata: WorkflowMetadata;
  triggers: WorkflowTrigger[];
  nodes: Record<string, WorkflowNode>;
  connections: WorkflowConnection[];
  tests?: WorkflowTest[];
  created_at: string;
  updated_at: string;
}

export interface ExecuteWorkflowRequest {
  trigger_data: Record<string, any>;
}

export type ExecutionStatus =
  | 'Pending'
  | 'Running'
  | 'Completed'
  | 'Failed'
  | 'Cancelled'
  | 'Timeout';

export interface ExecuteWorkflowResponse {
  execution_id: string;
  workflow_id: string;
  status: ExecutionStatus;
  outputs?: Record<string, any>;
  error?: string;
  started_at: string;
  completed_at?: string;
  duration_ms: number;
}

export interface GetExecutionResponse {
  execution_id: string;
  workflow_id: string;
  status: ExecutionStatus;
  outputs?: Record<string, any>;
  error?: string;
  started_at: string;
  completed_at?: string;
  duration_ms: number;
}

// Node types and schemas
export interface PropertySchema {
  property_type: string;
  description: string;
  default?: any;
  allowed_values?: any[];
  minimum?: number;
  maximum?: number;
  pattern?: string;
}

export interface SchemaConstraints {
  min_length?: number;
  max_length?: number;
  min_items?: number;
  max_items?: number;
  additional_properties: boolean;
}

export interface NodeSchema {
  schema_type: string;
  required: string[];
  properties: Record<string, PropertySchema>;
  constraints: SchemaConstraints;
}

export interface NodeExample {
  name: string;
  description: string;
  config: Record<string, any>;
  input: Record<string, any>;
  output: Record<string, any>;
}

export interface ResourceRequirements {
  memory_mb?: number;
  cpu_percent?: number;
  network_io: boolean;
  disk_io: boolean;
  external_dependencies: string[];
}

export interface NodeCapabilities {
  supports_streaming: boolean;
  supports_batch: boolean;
  cacheable: boolean;
  has_side_effects: boolean;
  idempotent: boolean;
  resource_requirements: ResourceRequirements;
}

export interface NodeDescription {
  node_type: string;
  description: string;
  inputs: NodeSchema;
  outputs: NodeSchema;
  config: NodeSchema;
  examples: NodeExample[];
}

export interface ListNodesResponse {
  nodes: string[];
  descriptions: NodeDescription[];
  total: number;
}

export enum NodeCategory {
  Transform = 'Transform',
  Http = 'Http',
  Database = 'Database',
  Control = 'Control',
  Validation = 'Validation',
  Communication = 'Communication',
  Storage = 'Storage',
  Custom = 'Custom',
}

// UI-specific types
export interface WorkflowListItem {
  id: string;
  name: string;
  description?: string;
  status: 'active' | 'inactive' | 'error';
  last_execution?: string;
  node_count: number;
  created_at: string;
  updated_at: string;
}

export interface ExecutionListItem {
  execution_id: string;
  workflow_id: string;
  workflow_name: string;
  status: ExecutionStatus;
  started_at: string;
  completed_at?: string;
  duration_ms: number;
  error?: string;
}

// React Flow types
export interface FlowNode extends WorkflowNode {
  data: {
    label: string;
    nodeType: string;
    config: Record<string, any>;
    description?: string;
  };
  type: string;
  position: { x: number; y: number };
  dragHandle?: string;
}

export interface FlowEdge extends WorkflowConnection {
  id: string;
  source: string;
  target: string;
  type?: string;
  animated?: boolean;
  style?: Record<string, any>;
  labelStyle?: Record<string, any>;
  labelBgStyle?: Record<string, any>;
}

// WebSocket types
export interface WebSocketMessage {
  type: 'execution_update' | 'workflow_update' | 'node_update';
  data: any;
}

export interface ExecutionUpdate {
  execution_id: string;
  workflow_id: string;
  status: ExecutionStatus;
  current_node?: string;
  completed_nodes: string[];
  error?: string;
  outputs?: Record<string, any>;
}

// Form types
export interface WorkflowFormData {
  name: string;
  description?: string;
  tags: string[];
  author?: string;
  organization?: string;
}

export interface NodeConfigFormData {
  [key: string]: any;
}

export interface TriggerFormData {
  type: 'webhook' | 'schedule' | 'event' | 'manual';
  [key: string]: any;
}

// Error types
export interface ApiError {
  message: string;
  code?: string;
  details?: any;
}

export interface ValidationError {
  field: string;
  message: string;
}
