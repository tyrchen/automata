import React, { useEffect, useState, useMemo } from 'react';
import { Link } from 'react-router-dom';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card';
import { Button } from '../components/ui/button';
import { Badge } from '../components/ui/badge';
import { Input } from '../components/ui/input';
import { DataTable } from '../components/ui/data-table';
import {
  Search,
  MoreHorizontal,
  Eye,
  RotateCcw,
  Clock,
  CheckCircle,
  XCircle,
  StopCircle,
  Play
} from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../components/ui/dropdown-menu';
import { ColumnDef } from '@tanstack/react-table';
import apiClient from '../services/api';
import { ExecutionListItem, ExecutionStatus } from '../types';

const ExecutionList: React.FC = () => {
  const [executions, setExecutions] = useState<ExecutionListItem[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');

  useEffect(() => {
    const loadExecutions = async () => {
      setIsLoading(true);
      try {
        const response = await apiClient.getExecutions();
        if (response.data) {
          setExecutions(response.data.executions);
        }
      } catch (error) {
        console.error('Failed to load executions:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadExecutions();
  }, []);

  const handleCancelExecution = async (id: string) => {
    try {
      await apiClient.cancelExecution(id);
      // Refresh the list
      const response = await apiClient.getExecutions();
      if (response.data) {
        setExecutions(response.data.executions);
      }
    } catch (error) {
      console.error('Failed to cancel execution:', error);
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

  const filteredExecutions = useMemo(() => {
    if (!searchTerm) return executions;

    return executions.filter(execution =>
      execution.workflow_name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      execution.execution_id.toLowerCase().includes(searchTerm.toLowerCase())
    );
  }, [executions, searchTerm]);

  const columns: ColumnDef<ExecutionListItem>[] = [
    {
      accessorKey: "workflow_name",
      header: "Workflow",
      cell: ({ row }) => {
        const execution = row.original;
        return (
          <div>
            <Link
              to={`/workflows/${execution.workflow_id}`}
              className="font-medium hover:underline"
            >
              {execution.workflow_name}
            </Link>
            <p className="text-sm text-muted-foreground">
              ID: <Link
                to={`/executions/${execution.execution_id}`}
                className="hover:underline"
              >
                {execution.execution_id.slice(0, 8)}...
              </Link>
            </p>
          </div>
        );
      },
    },
    {
      accessorKey: "status",
      header: "Status",
      cell: ({ row }) => {
        const status = row.getValue("status") as ExecutionStatus;
        return (
          <div className="flex items-center gap-2">
            {getStatusIcon(status)}
            {getStatusBadge(status)}
          </div>
        );
      },
    },
    {
      accessorKey: "started_at",
      header: "Started",
      cell: ({ row }) => {
        const date = new Date(row.getValue("started_at") as string);
        return (
          <div>
            <div className="text-sm">{date.toLocaleDateString()}</div>
            <div className="text-xs text-muted-foreground">
              {date.toLocaleTimeString()}
            </div>
          </div>
        );
      },
    },
    {
      accessorKey: "duration_ms",
      header: "Duration",
      cell: ({ row }) => {
        const duration = row.getValue("duration_ms") as number;
        const status = row.original.status;

        if (status === 'Running' || status === 'Pending') {
          return <span className="text-muted-foreground">—</span>;
        }

        return <span className="text-sm">{formatDuration(duration)}</span>;
      },
    },
    {
      accessorKey: "completed_at",
      header: "Completed",
      cell: ({ row }) => {
        const completedAt = row.getValue("completed_at") as string;
        if (!completedAt) {
          return <span className="text-muted-foreground">—</span>;
        }
        const date = new Date(completedAt);
        return (
          <div>
            <div className="text-sm">{date.toLocaleDateString()}</div>
            <div className="text-xs text-muted-foreground">
              {date.toLocaleTimeString()}
            </div>
          </div>
        );
      },
    },
    {
      id: "actions",
      cell: ({ row }) => {
        const execution = row.original;
        const canCancel = execution.status === 'Running' || execution.status === 'Pending';

        return (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" className="h-8 w-8 p-0">
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem asChild>
                <Link to={`/executions/${execution.execution_id}`}>
                  <Eye className="mr-2 h-4 w-4" />
                  View Details
                </Link>
              </DropdownMenuItem>
              <DropdownMenuItem asChild>
                <Link to={`/workflows/${execution.workflow_id}`}>
                  <Eye className="mr-2 h-4 w-4" />
                  View Workflow
                </Link>
              </DropdownMenuItem>
              {execution.status === 'Completed' && (
                <DropdownMenuItem>
                  <RotateCcw className="mr-2 h-4 w-4" />
                  Re-run
                </DropdownMenuItem>
              )}
              {canCancel && (
                <>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    onClick={() => handleCancelExecution(execution.execution_id)}
                    className="text-destructive"
                  >
                    <StopCircle className="mr-2 h-4 w-4" />
                    Cancel
                  </DropdownMenuItem>
                </>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        );
      },
    },
  ];

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Executions</h1>
          <p className="text-muted-foreground">
            Monitor and track your workflow execution history
          </p>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Executions</CardTitle>
            <Play className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{executions.length}</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Successful</CardTitle>
            <CheckCircle className="h-4 w-4 text-green-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-600">
              {executions.filter(e => e.status === 'Completed').length}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Failed</CardTitle>
            <XCircle className="h-4 w-4 text-red-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-red-600">
              {executions.filter(e => e.status === 'Failed').length}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Running</CardTitle>
            <Clock className="h-4 w-4 text-blue-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-blue-600">
              {executions.filter(e => e.status === 'Running').length}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Search and Filters */}
      <div className="flex items-center gap-4">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search executions..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="pl-10"
          />
        </div>
      </div>

      {/* Executions Table */}
      <Card>
        <CardHeader>
          <CardTitle>Execution History</CardTitle>
          <CardDescription>
            {filteredExecutions.length} execution{filteredExecutions.length !== 1 ? 's' : ''} found
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {[...Array(5)].map((_, i) => (
                <div key={i} className="flex items-center space-x-4">
                  <div className="h-4 bg-muted rounded flex-1 animate-pulse" />
                  <div className="h-4 bg-muted rounded w-20 animate-pulse" />
                  <div className="h-4 bg-muted rounded w-24 animate-pulse" />
                  <div className="h-4 bg-muted rounded w-16 animate-pulse" />
                </div>
              ))}
            </div>
          ) : filteredExecutions.length === 0 ? (
            <div className="text-center py-12">
              {searchTerm ? (
                <div>
                  <Search className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
                  <h3 className="text-lg font-medium mb-2">No executions found</h3>
                  <p className="text-muted-foreground">
                    No executions match your search criteria
                  </p>
                </div>
              ) : (
                <div>
                  <Play className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
                  <h3 className="text-lg font-medium mb-2">No executions yet</h3>
                  <p className="text-muted-foreground mb-4">
                    Execute a workflow to see execution history here
                  </p>
                  <Button asChild>
                    <Link to="/workflows">
                      View Workflows
                    </Link>
                  </Button>
                </div>
              )}
            </div>
          ) : (
            <DataTable
              columns={columns}
              data={filteredExecutions}
            />
          )}
        </CardContent>
      </Card>
    </div>
  );
};

export default ExecutionList;
