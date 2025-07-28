import React, { useState, useEffect, useCallback } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/card';
import { Button } from '../ui/button';
import { Alert, AlertDescription } from '../ui/alert';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../ui/tabs';
import {
  Code2,
  Eye,
  FileCode,
  Copy,
  Download,
  Upload,
  Play,
  AlertCircle,
  CheckCircle
} from 'lucide-react';
import { cn } from '../../lib/utils';

interface DslEditorProps {
  value: string;
  onChange: (value: string) => void;
  onParse: () => void;
  onExecute: () => void;
  error?: string;
  isValid?: boolean;
  className?: string;
}

const WORKFLOW_EXAMPLES = {
  simple: `metadata:
  name: "Simple Workflow"
  version: "1.0.0"
  description: "A basic workflow example"

triggers:
  - manual: {}

nodes:
  start:
    type: transformer
    mapping:
      message: "Hello, World!"
      timestamp: $now()

connections:
  - from: trigger
    to: start`,

  userRegistration: `metadata:
  name: "User Registration"
  version: "1.0.0"
  description: "Process new user registration"

triggers:
  - webhook:
      path: "/api/register"
      method: "POST"

nodes:
  validate_input:
    type: validator
    rules:
      - field: email
        type: string
        required: true
        pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
      - field: name
        type: string
        required: true
      - field: password
        type: string
        required: true
        min_length: 8

  hash_password:
    type: transformer
    mapping:
      email: $validate_input.data.email
      name: $validate_input.data.name
      password_hash: $hash($validate_input.data.password, "bcrypt")
      created_at: $now()
      user_id: $uuid()

  save_user:
    type: database
    operation: insert
    table: users
    data: $hash_password.output

  send_welcome_email:
    type: http
    method: POST
    url: "https://api.email.com/send"
    headers:
      Authorization: "Bearer $env(EMAIL_API_KEY)"
    body:
      to: $save_user.output.email
      subject: "Welcome to our platform!"
      template: "welcome"
      data:
        name: $save_user.output.name

connections:
  - from: trigger
    to: validate_input
  - from: validate_input
    to: hash_password
    condition: $validate_input.success
  - from: hash_password
    to: save_user
  - from: save_user
    to: send_welcome_email
    condition: $save_user.success`,

  dataProcessing: `metadata:
  name: "Data Processing Pipeline"
  version: "1.0.0"
  description: "ETL workflow for data processing"

triggers:
  - schedule:
      cron: "0 0 * * *"
      timezone: "UTC"

nodes:
  fetch_data:
    type: http
    method: GET
    url: "https://api.example.com/data"
    headers:
      Authorization: "Bearer $env(API_TOKEN)"

  validate_data:
    type: validator
    rules:
      - field: records
        type: array
        required: true
        min_items: 1

  transform_batch:
    type: forEach
    items: $fetch_data.output.records
    operation:
      type: transformer
      mapping:
        id: $item.id
        processed_name: $upper($item.name)
        timestamp: $now()
        category: $item.type

  save_results:
    type: database
    operation: bulk_insert
    table: processed_records
    data: $transform_batch.output

  send_report:
    type: http
    method: POST
    url: "https://slack.com/api/chat.postMessage"
    headers:
      Authorization: "Bearer $env(SLACK_TOKEN)"
    body:
      channel: "#data-reports"
      text: "Data processing complete: $len($save_results.output) records processed"

connections:
  - from: trigger
    to: fetch_data
  - from: fetch_data
    to: validate_data
  - from: validate_data
    to: transform_batch
    condition: $validate_data.success
  - from: transform_batch
    to: save_results
  - from: save_results
    to: send_report`,

  errorHandling: `metadata:
  name: "Error Handling Example"
  version: "1.0.0"
  description: "Workflow with retry and error handling"

triggers:
  - event:
      source: "payment-service"
      event_type: "payment.received"

nodes:
  process_payment:
    type: http
    method: POST
    url: "https://payment-processor.com/api/process"
    timeout: 30
    retry:
      max_attempts: 3
      delay_ms: 1000
      backoff_multiplier: 2
    body:
      payment_id: $trigger.data.payment_id
      amount: $trigger.data.amount

  update_order:
    type: database
    operation: update
    table: orders
    condition: "id = $trigger.data.order_id"
    data:
      payment_status: "completed"
      payment_id: $process_payment.output.transaction_id
      updated_at: $now()

  notify_customer:
    type: parallel
    branches:
      - email:
          type: http
          method: POST
          url: "https://api.email.com/send"
          body:
            to: $trigger.data.customer_email
            subject: "Payment Received"
            template: "payment_success"
      - sms:
          type: http
          method: POST
          url: "https://api.sms.com/send"
          body:
            to: $trigger.data.customer_phone
            message: "Your payment has been processed successfully"

  handle_error:
    type: http
    method: POST
    url: "https://api.alerts.com/notify"
    condition: $process_payment.error
    body:
      alert_type: "payment_failed"
      payment_id: $trigger.data.payment_id
      error: $process_payment.error_message

connections:
  - from: trigger
    to: process_payment
  - from: process_payment
    to: update_order
    condition: $process_payment.success
  - from: update_order
    to: notify_customer
  - from: process_payment
    to: handle_error
    condition: $process_payment.error`
};

export const DslEditor: React.FC<DslEditorProps> = ({
  value,
  onChange,
  onParse,
  onExecute,
  error,
  isValid,
  className
}) => {
  const [localValue, setLocalValue] = useState(value);
  const [selectedExample, setSelectedExample] = useState<string>('');

  useEffect(() => {
    setLocalValue(value);
  }, [value]);

  const handleChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    setLocalValue(newValue);
    onChange(newValue);
  }, [onChange]);

  const handleLoadExample = useCallback((exampleKey: string) => {
    const example = WORKFLOW_EXAMPLES[exampleKey as keyof typeof WORKFLOW_EXAMPLES];
    if (example) {
      setLocalValue(example);
      onChange(example);
      setSelectedExample(exampleKey);
    }
  }, [onChange]);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(localValue);
  }, [localValue]);

  const handleDownload = useCallback(() => {
    const blob = new Blob([localValue], { type: 'text/yaml' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'workflow.yaml';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [localValue]);

  const handleUpload = useCallback(() => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.yaml,.yml';
    input.onchange = (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (file) {
        const reader = new FileReader();
        reader.onload = (e) => {
          const content = e.target?.result as string;
          setLocalValue(content);
          onChange(content);
        };
        reader.readAsText(file);
      }
    };
    input.click();
  }, [onChange]);

  return (
    <div className={cn("space-y-4", className)}>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FileCode className="h-5 w-5" />
            Workflow DSL Editor
          </CardTitle>
          <CardDescription>
            Define your workflow using YAML DSL syntax
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Toolbar */}
          <div className="flex items-center justify-between gap-2 flex-wrap">
            <div className="flex items-center gap-2">
              <select
                value={selectedExample}
                onChange={(e) => handleLoadExample(e.target.value)}
                className="px-3 py-2 text-sm border rounded-md"
              >
                <option value="">Load Example...</option>
                <option value="simple">Simple Workflow</option>
                <option value="userRegistration">User Registration</option>
                <option value="dataProcessing">Data Processing Pipeline</option>
                <option value="errorHandling">Error Handling</option>
              </select>

              <Button variant="outline" size="sm" onClick={handleUpload}>
                <Upload className="h-4 w-4 mr-2" />
                Upload
              </Button>
            </div>

            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm" onClick={handleCopy}>
                <Copy className="h-4 w-4 mr-2" />
                Copy
              </Button>

              <Button variant="outline" size="sm" onClick={handleDownload}>
                <Download className="h-4 w-4 mr-2" />
                Download
              </Button>

              <Button
                variant="outline"
                size="sm"
                onClick={onParse}
                disabled={!localValue.trim()}
              >
                <Eye className="h-4 w-4 mr-2" />
                Preview
              </Button>

              <Button
                size="sm"
                onClick={onExecute}
                disabled={!isValid || !localValue.trim()}
              >
                <Play className="h-4 w-4 mr-2" />
                Execute
              </Button>
            </div>
          </div>

          {/* Status Alert */}
          {error && (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          {isValid && !error && localValue.trim() && (
            <Alert>
              <CheckCircle className="h-4 w-4" />
              <AlertDescription>Workflow DSL is valid</AlertDescription>
            </Alert>
          )}

          {/* Editor */}
          <div className="relative">
            <textarea
              value={localValue}
              onChange={handleChange}
              className={cn(
                "w-full min-h-[400px] p-4 font-mono text-sm",
                "border rounded-lg bg-muted/30",
                "focus:outline-none focus:ring-2 focus:ring-ring",
                "placeholder:text-muted-foreground",
                error && "border-destructive"
              )}
              placeholder={`metadata:
  name: "My Workflow"
  version: "1.0.0"
  description: "Workflow description"

triggers:
  - manual: {}

nodes:
  my_node:
    type: transformer
    mapping:
      message: "Hello"

connections:
  - from: trigger
    to: my_node`}
              spellCheck={false}
            />

            {/* Line numbers (optional enhancement) */}
            <div className="absolute left-0 top-0 p-4 select-none pointer-events-none">
              <div className="text-xs text-muted-foreground font-mono">
                {localValue.split('\n').map((_, i) => (
                  <div key={i} className="h-[1.5em]">{i + 1}</div>
                ))}
              </div>
            </div>
          </div>

          {/* Syntax Help */}
          <Tabs defaultValue="structure" className="mt-4">
            <TabsList>
              <TabsTrigger value="structure">Structure</TabsTrigger>
              <TabsTrigger value="nodes">Node Types</TabsTrigger>
              <TabsTrigger value="expressions">Expressions</TabsTrigger>
            </TabsList>

            <TabsContent value="structure" className="space-y-2 text-sm">
              <p className="text-muted-foreground">Basic workflow structure:</p>
              <pre className="p-3 bg-muted rounded-md overflow-x-auto">
{`metadata:           # Workflow metadata
  name: string      # Required
  version: string   # Required
  description: string

triggers:           # How workflow is triggered
  - manual: {}      # Manual trigger
  - webhook:        # HTTP webhook
      path: string
      method: string
  - schedule:       # Cron schedule
      cron: string
      timezone: string

nodes:              # Workflow nodes
  node_id:
    type: string    # Node type
    # Node-specific config

connections:        # Node connections
  - from: string    # Source node ID
    to: string      # Target node ID
    condition: expr # Optional condition`}
              </pre>
            </TabsContent>

            <TabsContent value="nodes" className="space-y-2 text-sm">
              <p className="text-muted-foreground">Available node types:</p>
              <div className="grid gap-2">
                <div className="p-3 bg-muted rounded-md">
                  <strong>transformer</strong> - Transform data using mappings
                  <pre className="mt-1 text-xs">{`type: transformer
mapping:
  key: value or $expression`}</pre>
                </div>
                <div className="p-3 bg-muted rounded-md">
                  <strong>validator</strong> - Validate input data
                  <pre className="mt-1 text-xs">{`type: validator
rules:
  - field: string
    type: string
    required: boolean`}</pre>
                </div>
                <div className="p-3 bg-muted rounded-md">
                  <strong>http</strong> - Make HTTP requests
                  <pre className="mt-1 text-xs">{`type: http
method: GET/POST/PUT/DELETE
url: string
headers: object
body: object`}</pre>
                </div>
                <div className="p-3 bg-muted rounded-md">
                  <strong>database</strong> - Database operations
                  <pre className="mt-1 text-xs">{`type: database
operation: insert/update/delete/query
table: string
data: object`}</pre>
                </div>
              </div>
            </TabsContent>

            <TabsContent value="expressions" className="space-y-2 text-sm">
              <p className="text-muted-foreground">Expression functions:</p>
              <div className="grid gap-2">
                <code className="p-2 bg-muted rounded">$now() - Current timestamp</code>
                <code className="p-2 bg-muted rounded">$uuid() - Generate UUID</code>
                <code className="p-2 bg-muted rounded">$env(KEY) - Environment variable</code>
                <code className="p-2 bg-muted rounded">$node.output - Node output reference</code>
                <code className="p-2 bg-muted rounded">$trigger.data - Trigger data</code>
                <code className="p-2 bg-muted rounded">$len(array) - Array length</code>
                <code className="p-2 bg-muted rounded">$upper(string) - Uppercase</code>
                <code className="p-2 bg-muted rounded">$hash(value, algorithm) - Hash value</code>
              </div>
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>
    </div>
  );
};

export default DslEditor;
