import React, { useEffect, useState, useMemo } from 'react';
import { Link } from 'react-router-dom';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card';
import { Button } from '../components/ui/button';
import { Badge } from '../components/ui/badge';
import { Input } from '../components/ui/input';
import { DataTable } from '../components/ui/data-table';
import {
  Plus,
  Search,
  MoreHorizontal,
  Edit,
  Trash2,
  Play,
  Eye,
  Copy
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
import { WorkflowListItem } from '../types';

const WorkflowList: React.FC = () => {
  const [workflows, setWorkflows] = useState<WorkflowListItem[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');

  useEffect(() => {
    const loadWorkflows = async () => {
      setIsLoading(true);
      try {
        const response = await apiClient.getWorkflows();
        if (response.data) {
          setWorkflows(response.data.workflows);
        }
      } catch (error) {
        console.error('Failed to load workflows:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadWorkflows();
  }, []);

  const handleDeleteWorkflow = async (id: string) => {
    if (!confirm('Are you sure you want to delete this workflow?')) return;

    try {
      await apiClient.deleteWorkflow(id);
      setWorkflows(workflows.filter(w => w.id !== id));
    } catch (error) {
      console.error('Failed to delete workflow:', error);
    }
  };

  const handleExecuteWorkflow = async (id: string) => {
    try {
      await apiClient.executeWorkflow(id, { trigger_data: {} });
      // Could show success message or redirect to executions
    } catch (error) {
      console.error('Failed to execute workflow:', error);
    }
  };

  const filteredWorkflows = useMemo(() => {
    if (!searchTerm) return workflows;

    return workflows.filter(workflow =>
      workflow.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      workflow.description?.toLowerCase().includes(searchTerm.toLowerCase())
    );
  }, [workflows, searchTerm]);

  const columns: ColumnDef<WorkflowListItem>[] = [
    {
      accessorKey: "name",
      header: "Name",
      cell: ({ row }) => {
        const workflow = row.original;
        return (
          <div>
            <Link
              to={`/workflows/${workflow.id}`}
              className="font-medium hover:underline"
            >
              {workflow.name}
            </Link>
            {workflow.description && (
              <p className="text-sm text-muted-foreground">
                {workflow.description}
              </p>
            )}
          </div>
        );
      },
    },
    {
      accessorKey: "status",
      header: "Status",
      cell: ({ row }) => {
        const status = row.getValue("status") as string;
        return (
          <Badge variant={status === 'active' ? 'default' : 'secondary'}>
            {status}
          </Badge>
        );
      },
    },
    {
      accessorKey: "node_count",
      header: "Nodes",
      cell: ({ row }) => {
        return <span className="text-muted-foreground">{row.getValue("node_count")}</span>;
      },
    },
    {
      accessorKey: "last_execution",
      header: "Last Execution",
      cell: ({ row }) => {
        const lastExecution = row.getValue("last_execution") as string;
        return lastExecution ? (
          <span className="text-sm">
            {new Date(lastExecution).toLocaleDateString()}
          </span>
        ) : (
          <span className="text-muted-foreground text-sm">Never</span>
        );
      },
    },
    {
      accessorKey: "updated_at",
      header: "Updated",
      cell: ({ row }) => {
        const date = new Date(row.getValue("updated_at") as string);
        return <span className="text-sm">{date.toLocaleDateString()}</span>;
      },
    },
    {
      id: "actions",
      cell: ({ row }) => {
        const workflow = row.original;

        return (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" className="h-8 w-8 p-0">
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem asChild>
                <Link to={`/workflows/${workflow.id}`}>
                  <Eye className="mr-2 h-4 w-4" />
                  View
                </Link>
              </DropdownMenuItem>
              <DropdownMenuItem asChild>
                <Link to={`/workflows/${workflow.id}/edit`}>
                  <Edit className="mr-2 h-4 w-4" />
                  Edit
                </Link>
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => handleExecuteWorkflow(workflow.id)}>
                <Play className="mr-2 h-4 w-4" />
                Execute
              </DropdownMenuItem>
              <DropdownMenuItem>
                <Copy className="mr-2 h-4 w-4" />
                Duplicate
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onClick={() => handleDeleteWorkflow(workflow.id)}
                className="text-destructive"
              >
                <Trash2 className="mr-2 h-4 w-4" />
                Delete
              </DropdownMenuItem>
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
          <h1 className="text-3xl font-bold">Workflows</h1>
          <p className="text-muted-foreground">
            Manage and monitor your automated workflows
          </p>
        </div>
        <Button asChild>
          <Link to="/workflows/new">
            <Plus className="h-4 w-4 mr-2" />
            New Workflow
          </Link>
        </Button>
      </div>

      {/* Search and Filters */}
      <div className="flex items-center gap-4">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search workflows..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="pl-10"
          />
        </div>
      </div>

      {/* Workflows Table */}
      <Card>
        <CardHeader>
          <CardTitle>All Workflows</CardTitle>
          <CardDescription>
            {filteredWorkflows.length} workflow{filteredWorkflows.length !== 1 ? 's' : ''} found
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-4">
              {[...Array(5)].map((_, i) => (
                <div key={i} className="flex items-center space-x-4">
                  <div className="h-4 bg-muted rounded flex-1 animate-pulse" />
                  <div className="h-4 bg-muted rounded w-20 animate-pulse" />
                  <div className="h-4 bg-muted rounded w-16 animate-pulse" />
                  <div className="h-4 bg-muted rounded w-24 animate-pulse" />
                </div>
              ))}
            </div>
          ) : filteredWorkflows.length === 0 ? (
            <div className="text-center py-12">
              {searchTerm ? (
                <div>
                  <Search className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
                  <h3 className="text-lg font-medium mb-2">No workflows found</h3>
                  <p className="text-muted-foreground">
                    No workflows match your search criteria
                  </p>
                </div>
              ) : (
                <div>
                  <div className="h-12 w-12 mx-auto mb-4 rounded-lg bg-muted flex items-center justify-center">
                    <Plus className="h-6 w-6 text-muted-foreground" />
                  </div>
                  <h3 className="text-lg font-medium mb-2">No workflows yet</h3>
                  <p className="text-muted-foreground mb-4">
                    Get started by creating your first workflow
                  </p>
                  <Button asChild>
                    <Link to="/workflows/new">
                      <Plus className="h-4 w-4 mr-2" />
                      Create Workflow
                    </Link>
                  </Button>
                </div>
              )}
            </div>
          ) : (
            <DataTable
              columns={columns}
              data={filteredWorkflows}
            />
          )}
        </CardContent>
      </Card>
    </div>
  );
};

export default WorkflowList;
