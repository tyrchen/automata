-- Add migration script here
-- Initial schema creation for Automata workflow engine
-- Create workflows table
CREATE TABLE IF NOT EXISTS workflows(
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  name varchar(255) NOT NULL,
  description text,
  definition text NOT NULL,
  version varchar(50) NOT NULL DEFAULT '1.0.0',
  status varchar(50) NOT NULL DEFAULT 'active',
  created_at timestamptz NOT NULL DEFAULT NOW(),
  updated_at timestamptz NOT NULL DEFAULT NOW(),
  created_by varchar(255),
  tags text[] DEFAULT '{}',
  metadata jsonb DEFAULT '{}'
);

-- Create index on workflow name and status
CREATE INDEX idx_workflows_name ON workflows(name);

CREATE INDEX idx_workflows_status ON workflows(status);

CREATE INDEX idx_workflows_created_at ON workflows(created_at);

-- Create executions table
CREATE TABLE IF NOT EXISTS executions(
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  workflow_id uuid NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
  status varchar(50) NOT NULL DEFAULT 'pending',
  trigger_data jsonb DEFAULT '{}',
  context jsonb DEFAULT '{}',
  outputs jsonb DEFAULT '{}',
  error text,
  started_at timestamptz NOT NULL DEFAULT NOW(),
  completed_at timestamptz,
  duration_ms bigint,
  created_at timestamptz NOT NULL DEFAULT NOW(),
  metadata jsonb DEFAULT '{}'
);

-- Create indexes on executions
CREATE INDEX idx_executions_workflow_id ON executions(workflow_id);

CREATE INDEX idx_executions_status ON executions(status);

CREATE INDEX idx_executions_started_at ON executions(started_at);

-- Create execution_logs table for detailed logging
CREATE TABLE IF NOT EXISTS execution_logs(
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  execution_id uuid NOT NULL REFERENCES executions(id) ON DELETE CASCADE,
  node_id varchar(255) NOT NULL,
  level varchar(20) NOT NULL DEFAULT 'info',
  message text NOT NULL,
  data jsonb DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT NOW()
);

-- Create index on execution logs
CREATE INDEX idx_execution_logs_execution_id ON execution_logs(execution_id);

CREATE INDEX idx_execution_logs_created_at ON execution_logs(created_at);

-- Create workflow_versions table for version history
CREATE TABLE IF NOT EXISTS workflow_versions(
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  workflow_id uuid NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
  version varchar(50) NOT NULL,
  definition text NOT NULL,
  change_description text,
  created_at timestamptz NOT NULL DEFAULT NOW(),
  created_by varchar(255)
);

-- Create index on workflow versions
CREATE INDEX idx_workflow_versions_workflow_id ON workflow_versions(workflow_id);

CREATE INDEX idx_workflow_versions_created_at ON workflow_versions(created_at);

-- Create trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
  RETURNS TRIGGER
  AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$
LANGUAGE 'plpgsql';

CREATE TRIGGER update_workflows_updated_at
  BEFORE UPDATE ON workflows
  FOR EACH ROW
  EXECUTE FUNCTION update_updated_at_column();

-- Create function to calculate execution duration
CREATE OR REPLACE FUNCTION calculate_execution_duration()
  RETURNS TRIGGER
  AS $$
BEGIN
  IF NEW.completed_at IS NOT NULL AND NEW.started_at IS NOT NULL THEN
    NEW.duration_ms = EXTRACT(EPOCH FROM(NEW.completed_at - NEW.started_at)) * 1000;
  END IF;
  RETURN NEW;
END;
$$
LANGUAGE 'plpgsql';

CREATE TRIGGER calculate_duration
  BEFORE INSERT OR UPDATE ON executions
  FOR EACH ROW
  EXECUTE FUNCTION calculate_execution_duration();
