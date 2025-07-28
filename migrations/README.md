# Automata Workflow Engine - Database Migrations

This directory contains SQLx migrations for the Automata workflow engine. These migrations set up the database schema and populate it with demo workflows.

## Migration Files

1. **20240115_000001_create_initial_schema.sql**
   - Creates the core database tables:
     - `workflows` - Stores workflow definitions
     - `executions` - Tracks workflow execution history
     - `execution_logs` - Detailed logs for each execution
     - `workflow_versions` - Version history for workflows
   - Sets up indexes for performance
   - Creates triggers for automatic timestamp updates and duration calculation

2. **20240115_000002_insert_demo_workflows.sql**
   - Inserts 4 basic demo workflows:
     - Hello World Demo - Simple workflow demonstration
     - User Registration Demo - Form validation and email sending
     - Daily Data Processing - ETL pipeline with scheduling
     - Payment Processing - Error handling and retry logic

3. **20240115_000003_insert_advanced_workflows.sql**
   - Inserts 4 advanced demo workflows:
     - Document Approval Workflow - Multi-level approval process
     - Cross-System Data Sync - Data synchronization with conflict resolution
     - ML Model Training Pipeline - Automated machine learning workflow
     - Automated Incident Response - Incident detection and auto-remediation

## Running Migrations

### Using SQLx CLI

1. Install SQLx CLI if you haven't already:
   ```bash
   cargo install sqlx-cli --no-default-features --features postgres
   ```

2. Set your database URL:
   ```bash
   export DATABASE_URL="postgres://username:password@localhost/automata"
   ```

3. Create the database (if it doesn't exist):
   ```bash
   sqlx database create
   ```

4. Run migrations:
   ```bash
   sqlx migrate run
   ```

### Using Cargo

From the project root directory:

```bash
cargo sqlx migrate run
```

### Manual Execution

If you prefer to run the migrations manually:

```bash
psql -U username -d automata -f migrations/20240115_000001_create_initial_schema.sql
psql -U username -d automata -f migrations/20240115_000002_insert_demo_workflows.sql
psql -U username -d automata -f migrations/20240115_000003_insert_advanced_workflows.sql
```

## Verifying Installation

After running the migrations, you can verify the installation:

```sql
-- Check workflows
SELECT id, name, status FROM workflows;

-- Check recent executions
SELECT w.name, e.status, e.started_at
FROM executions e
JOIN workflows w ON e.workflow_id = w.id
ORDER BY e.started_at DESC
LIMIT 10;
```

## Demo Workflows Overview

### Basic Workflows

1. **Hello World Demo** - Learn the basics with a simple transformation workflow
2. **User Registration Demo** - See validation, database operations, and HTTP calls
3. **Daily Data Processing** - Understand scheduling, parallel processing, and aggregation
4. **Payment Processing** - Explore error handling, retries, and conditional logic

### Advanced Workflows

5. **Document Approval Workflow** - Multi-step approval with notifications and escalation
6. **Cross-System Data Sync** - Complex data synchronization with conflict resolution
7. **ML Model Training Pipeline** - Complete ML workflow from data loading to deployment
8. **Automated Incident Response** - Incident detection, triage, and auto-remediation

## Testing Workflows

Once migrations are complete, you can:

1. Navigate to the UI at http://localhost:3000
2. View all demo workflows in the dashboard
3. Click on any workflow to see its visual representation
4. Switch to DSL Editor tab to see the YAML definition
5. Execute workflows using the Execute button
6. Monitor execution progress in the Executions page

## Customizing Workflows

Each workflow demonstrates different capabilities:

- **Triggers**: Manual, webhook, scheduled, event-based
- **Node Types**: Transformer, validator, HTTP, database, conditional, forEach, parallel
- **Patterns**: Error handling, retries, parallel execution, conditional branching
- **Integrations**: External APIs, databases, notification services

Feel free to modify these workflows or create your own using the DSL editor!
