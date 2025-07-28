import React, { useEffect, useState, useCallback } from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { Button } from '../components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card';
import { Badge } from '../components/ui/badge';
import { Separator } from '../components/ui/separator';
import { ScrollArea } from '../components/ui/scroll-area';
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from '../components/ui/resizable';
import {
  ArrowLeft,
  Clock,
  CheckCircle,
  XCircle,
  StopCircle,
  Play,
  AlertCircle,
  RefreshCw,
  Eye,
  Terminal,
  Activity
} from 'lucide-react';
import WorkflowCanvas from '../components/workflow/WorkflowCanvas';
import apiClient from '../services/api';
import { ExecutionStatus, GetExecutionResponse } from '../types';

interface ExecutionLogEntry {
  timestamp: string;
  level: 'info' | 'warn' | 'error' | 'debug';
  message: string;
  node_id?: string;
  context?: Record<string, any>;
}

interface ExecutionProgress {
  current_node?: string;
  completed_nodes: string[];
  failed_nodes: string[];
  total_nodes: number;
  progress_percentage: number;
}

const ExecutionDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const [execution, setExecution] = useState<GetExecutionResponse | null>(null);
  const [logs, setLogs] = useState<ExecutionLogEntry[]>([]);
  const [progress, setProgress] = useState<ExecutionProgress | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [autoRefresh, setAutoRefresh] = useState(false);
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date());

  useEffect(() => {
    const loadExecution = async () => {
      if (!id) return;

      setIsLoading(true);
      try {
        // Load execution details
        const executionResponse = await apiClient.getExecution(id);
        if (executionResponse.data) {
          setExecution(executionResponse.data);
        }

        // Load execution logs (simulated for now - would come from backend)
        const mockLogs: ExecutionLogEntry[] = [
          {
            timestamp: new Date().toISOString(),
            level: 'info',
            message: 'Execution started',
            context: { execution_id: id }
          },
          {
            timestamp: new Date(Date.now() - 5000).toISOString(),
            level: 'info',
            message: 'Processing node: validate_input',
            node_id: 'validate_input'
          },
          {
            timestamp: new Date(Date.now() - 3000).toISOString(),
            level: 'info',
            message: 'Node validate_input completed successfully',
            node_id: 'validate_input',
            context: { duration_ms: 245 }
          },
          {
            timestamp: new Date(Date.now() - 2000).toISOString(),
            level: 'info',
            message: 'Processing node: transform_data',
            node_id: 'transform_data'
          }
        ];

        const mockProgress: ExecutionProgress = {
          current_node: 'transform_data',
          completed_nodes: ['validate_input'],
          failed_nodes: [],
          total_nodes: 4,
          progress_percentage: 25
        };

        setLogs(mockLogs);
        setProgress(mockProgress);

      } catch (error) {
        console.error('Failed to load execution:', error);
        navigate('/executions');
      } finally {
        setIsLoading(false);
        setLastRefresh(new Date());
      }
    };

    loadExecution();
  }, [id, navigate]);

  // Auto-refresh for running executions
  useEffect(() => {
    if (!autoRefresh || !execution || !['Running', 'Pending'].includes(execution.status)) {
      return;
    }

    const interval = setInterval(() => {
      // Refresh execution data
      if (id) {
        apiClient.getExecution(id).then(response => {
          if (response.data) {
            setExecution(response.data);
            setLastRefresh(new Date());
          }
        }).catch(console.error);
      }
    }, 5000); // Refresh every 5 seconds

    return () => clearInterval(interval);
  }, [autoRefresh, execution, id]);

  const handleCancelExecution = async () => {
    if (!id) return;

    try {
      await apiClient.cancelExecution(id);
      // Refresh execution status
      const response = await apiClient.getExecution(id);
      if (response.data) {
        setExecution(response.data);
      }
    } catch (error) {
      console.error('Failed to cancel execution:', error);
      alert('Failed to cancel execution');
    }
  };

  const handleRerunExecution = async () => {
    if (!execution?.workflow_id) return;

    try {
      const response = await apiClient.executeWorkflow(execution.workflow_id, { trigger_data: {} });
      if (response.data) {
        navigate(`/executions/${response.data.execution_id}`);
      }
    } catch (error) {
      console.error('Failed to rerun execution:', error);
      alert('Failed to rerun execution');
    }
  };

  const getStatusIcon = (status: ExecutionStatus) => {
    switch (status) {
      case 'Completed':
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case 'Running':
        return <Clock className="h-4 w-4 text-blue-500" />;
      case 'Failed':
        return <XCircle className="h-4 w-4 text-red-500" />;
      case 'Cancelled':
        return <StopCircle className="h-4 w-4 text-gray-500" />;
      case 'Pending':
        return <Clock className="h-4 w-4 text-yellow-500" />;
      default:
        return <Clock className="h-4 w-4 text-gray-500" />;
    }
  };

  const getStatusBadge = (status: ExecutionStatus) => {
    const variants = {
      'Completed': 'default',
      'Running': 'secondary',
      'Failed': 'destructive',
      'Cancelled': 'outline',
      'Pending': 'secondary',
      'Timeout': 'destructive',
    } as const;

    return (
      <Badge variant={variants[status] || 'outline'}>
        {status}
      </Badge>
    );
  };

  const getLogLevelIcon = (level: ExecutionLogEntry['level']) => {
    switch (level) {
      case 'error':
        return <XCircle className="h-3 w-3 text-red-500" />;
      case 'warn':
        return <AlertCircle className="h-3 w-3 text-yellow-500" />;
      case 'info':
        return <CheckCircle className="h-3 w-3 text-blue-500" />;
      case 'debug':
        return <Terminal className="h-3 w-3 text-gray-500" />;
      default:
        return <Terminal className="h-3 w-3 text-gray-500" />;
    }
  };

  const formatDuration = (durationMs: number) => {
    if (durationMs < 1000) {
      return `${durationMs}ms`;
    }
    const seconds = Math.floor(durationMs / 1000);
    if (seconds < 60) {
      return `${seconds}s`;
    }
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds}s`;
  };

  if (isLoading) {
    return (
      <div className="p-6">
        <div className="animate-pulse space-y-4">
          <div className="h-8 bg-muted rounded-md w-1/3" />
          <div className="h-4 bg-muted rounded-md w-1/2" />
          <div className="h-64 bg-muted rounded-md" />
        </div>
      </div>
    );
  }

  if (!execution) {
    return (
      <div className="p-6">
        <Card>
          <CardContent className="pt-6">
            <div className="text-center">
              <AlertCircle className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
              <h3 className="text-lg font-medium mb-2">Execution not found</h3>
              <p className="text-muted-foreground mb-4">
                The execution you're looking for doesn't exist or has been deleted.
              </p>
              <Button asChild>
                <Link to="/executions">
                  Back to Executions
                </Link>
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  const canCancel = execution.status === 'Running' || execution.status === 'Pending';
  const canRerun = execution.status === 'Completed' || execution.status === 'Failed' || execution.status === 'Cancelled';

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="border-b border-border bg-background/95 backdrop-blur">
        <div className="px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => navigate('/executions')}
              >
                <ArrowLeft className="h-4 w-4 mr-2" />
                Back to Executions
              </Button>

              <div className="flex flex-col">
                <div className="flex items-center gap-3">
                  <h1 className="text-xl font-semibold">
                    Execution Details
                  </h1>
                  {getStatusIcon(execution.status)}
                  {getStatusBadge(execution.status)}
                </div>
                <p className="text-sm text-muted-foreground">
                  ID: {execution.execution_id} • Workflow: {' '}
                  <Link
                    to={`/workflows/${execution.workflow_id}`}
                    className="hover:underline"
                  >
                    View Workflow
                  </Link>
                </p>
              </div>
            </div>

            <div className="flex items-center gap-2">
              {(execution.status === 'Running' || execution.status === 'Pending') && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setAutoRefresh(!autoRefresh)}
                >
                  <RefreshCw className={`h-4 w-4 mr-2 ${autoRefresh ? 'animate-spin' : ''}`} />
                  {autoRefresh ? 'Auto-refresh On' : 'Auto-refresh Off'}
                </Button>
              )}

              {canCancel && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleCancelExecution}
                >
                  <StopCircle className="h-4 w-4 mr-2" />
                  Cancel
                </Button>
              )}

              {canRerun && (
                <Button
                  size="sm"
                  onClick={handleRerunExecution}
                >
                  <Play className="h-4 w-4 mr-2" />
                  Re-run
                </Button>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden">
        <ResizablePanelGroup direction="vertical">
          {/* Top Panel - Execution Info & Progress */}
          <ResizablePanel defaultSize={30} minSize={20}>
            <div className="p-6 space-y-6">
              {/* Execution Info Cards */}
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
                <Card>
                  <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                    <CardTitle className="text-sm font-medium">Status</CardTitle>
                    {getStatusIcon(execution.status)}
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold">{execution.status}</div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                    <CardTitle className="text-sm font-medium">Duration</CardTitle>
                    <Clock className="h-4 w-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold">
                      {execution.status === 'Running' || execution.status === 'Pending'
                        ? '—'
                        : formatDuration(execution.duration_ms)
                      }
                    </div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                    <CardTitle className="text-sm font-medium">Progress</CardTitle>
                    <Activity className="h-4 w-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className="text-2xl font-bold">
                      {progress ? `${progress.progress_percentage}%` : '—'}
                    </div>
                    {progress && (
                      <div className="text-xs text-muted-foreground mt-1">
                        {progress.completed_nodes.length} of {progress.total_nodes} nodes
                      </div>
                    )}
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                    <CardTitle className="text-sm font-medium">Started</CardTitle>
                    <Clock className="h-4 w-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className="text-sm font-medium">
                      {new Date(execution.started_at).toLocaleDateString()}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {new Date(execution.started_at).toLocaleTimeString()}
                    </div>
                  </CardContent>
                </Card>
              </div>

              {/* Error Message */}
              {execution.error && (
                <Card className="border-destructive">
                  <CardHeader>
                    <CardTitle className="text-destructive flex items-center gap-2">
                      <XCircle className="h-4 w-4" />
                      Execution Error
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <pre className="text-sm bg-muted p-3 rounded-md overflow-x-auto">
                      {execution.error}
                    </pre>
                  </CardContent>
                </Card>
              )}
            </div>
          </ResizablePanel>

          <ResizableHandle />

          {/* Bottom Panel - Logs & Visual */}
          <ResizablePanel defaultSize={70} minSize={40}>
            <ResizablePanelGroup direction="horizontal">
              {/* Logs Panel */}
              <ResizablePanel defaultSize={60} minSize={30}>
                <div className="h-full flex flex-col">
                  <div className="px-4 py-2 border-b bg-muted/50">
                    <div className="flex items-center justify-between">
                      <h3 className="text-sm font-medium flex items-center gap-2">
                        <Terminal className="h-4 w-4" />
                        Execution Logs
                      </h3>
                      <div className="text-xs text-muted-foreground">
                        Last updated: {lastRefresh.toLocaleTimeString()}
                      </div>
                    </div>
                  </div>
                  <ScrollArea className="flex-1">
                    <div className="p-4 space-y-2">
                      {logs.map((log, index) => (
                        <div key={index} className="flex gap-3 text-sm">
                          <div className="flex-shrink-0 mt-0.5">
                            {getLogLevelIcon(log.level)}
                          </div>
                          <div className="flex-shrink-0 text-xs text-muted-foreground font-mono min-w-[80px]">
                            {new Date(log.timestamp).toLocaleTimeString()}
                          </div>
                          <div className="flex-1">
                            <div className="flex items-center gap-2">
                              <span>{log.message}</span>
                              {log.node_id && (
                                <Badge variant="outline" className="text-xs">
                                  {log.node_id}
                                </Badge>
                              )}
                            </div>
                            {log.context && (
                              <pre className="text-xs text-muted-foreground mt-1 bg-muted/50 p-2 rounded">
                                {JSON.stringify(log.context, null, 2)}
                              </pre>
                            )}
                          </div>
                        </div>
                      ))}
                    </div>
                  </ScrollArea>
                </div>
              </ResizablePanel>

              <ResizableHandle />

              {/* Visual Progress Panel */}
              <ResizablePanel defaultSize={40} minSize={30}>
                <div className="h-full flex flex-col">
                  <div className="px-4 py-2 border-b bg-muted/50">
                    <h3 className="text-sm font-medium flex items-center gap-2">
                      <Eye className="h-4 w-4" />
                      Workflow Progress
                    </h3>
                  </div>
                  <div className="flex-1 relative">
                    <WorkflowCanvas
                      onSave={() => {}}
                      onExecute={() => {}}
                      readOnly={true}
                      executionProgress={progress}
                    />
                  </div>
                </div>
              </ResizablePanel>
            </ResizablePanelGroup>
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    </div>
  );
};

export default ExecutionDetail;
