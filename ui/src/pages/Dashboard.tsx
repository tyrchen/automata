import React, { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card';
import { Button } from '../components/ui/button';
import { Badge } from '../components/ui/badge';
import {
  Workflow,
  Play,
  Clock,
  CheckCircle,
  XCircle,
  Plus,
  TrendingUp,
  Activity
} from 'lucide-react';
import apiClient from '../services/api';
import { WorkflowListItem, ExecutionListItem } from '../types';

const Dashboard: React.FC = () => {
  const [workflows, setWorkflows] = useState<WorkflowListItem[]>([]);
  const [recentExecutions, setRecentExecutions] = useState<ExecutionListItem[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const loadDashboardData = async () => {
      setIsLoading(true);
      try {
        // Load recent workflows
        const workflowsResponse = await apiClient.getWorkflows({ limit: 5 });
        if (workflowsResponse.data) {
          setWorkflows(workflowsResponse.data.workflows);
        }

        // Load recent executions
        const executionsResponse = await apiClient.getExecutions({ limit: 10 });
        if (executionsResponse.data) {
          setRecentExecutions(executionsResponse.data.executions);
        }
      } catch (error) {
        console.error('Failed to load dashboard data:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadDashboardData();
  }, []);

  const getStatusBadge = (status: string) => {
    const variants = {
      'Completed': 'default',
      'Running': 'secondary',
      'Failed': 'destructive',
      'Cancelled': 'outline',
      'Pending': 'secondary',
    } as const;

    return (
      <Badge variant={variants[status as keyof typeof variants] || 'outline'}>
        {status}
      </Badge>
    );
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Completed':
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case 'Running':
        return <Clock className="h-4 w-4 text-blue-500" />;
      case 'Failed':
        return <XCircle className="h-4 w-4 text-red-500" />;
      default:
        return <Clock className="h-4 w-4 text-gray-500" />;
    }
  };

  if (isLoading) {
    return (
      <div className="p-6">
        <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4">
          {[...Array(4)].map((_, i) => (
            <Card key={i}>
              <CardHeader className="space-y-0 pb-2">
                <div className="h-4 bg-muted rounded animate-pulse" />
                <div className="h-8 bg-muted rounded animate-pulse" />
              </CardHeader>
            </Card>
          ))}
        </div>
      </div>
    );
  }

  const stats = {
    totalWorkflows: workflows.length,
    activeWorkflows: workflows.filter(w => w.status === 'active').length,
    recentExecutions: recentExecutions.length,
    successRate: recentExecutions.length > 0
      ? Math.round((recentExecutions.filter(e => e.status === 'Completed').length / recentExecutions.length) * 100)
      : 0
  };

  return (
    <div className="p-6 space-y-6">
      {/* Welcome Section */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Welcome back!</h1>
          <p className="text-muted-foreground">
            Here's what's happening with your workflows today.
          </p>
        </div>
        <Button asChild>
          <Link to="/workflows/new">
            <Plus className="h-4 w-4 mr-2" />
            New Workflow
          </Link>
        </Button>
      </div>

      {/* Stats Cards */}
      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Workflows</CardTitle>
            <Workflow className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats.totalWorkflows}</div>
            <p className="text-xs text-muted-foreground">
              {stats.activeWorkflows} active
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Recent Executions</CardTitle>
            <Play className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats.recentExecutions}</div>
            <p className="text-xs text-muted-foreground">
              Last 10 executions
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Success Rate</CardTitle>
            <TrendingUp className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats.successRate}%</div>
            <p className="text-xs text-muted-foreground">
              From recent executions
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">System Status</CardTitle>
            <Activity className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-600">Healthy</div>
            <p className="text-xs text-muted-foreground">
              All systems operational
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Recent Content */}
      <div className="grid gap-6 md:grid-cols-2">
        {/* Recent Workflows */}
        <Card>
          <CardHeader>
            <CardTitle>Recent Workflows</CardTitle>
            <CardDescription>
              Your most recently created or updated workflows
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {workflows.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  <Workflow className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>No workflows yet</p>
                  <Button asChild className="mt-2" variant="outline">
                    <Link to="/workflows/new">Create your first workflow</Link>
                  </Button>
                </div>
              ) : (
                workflows.map((workflow) => (
                  <div key={workflow.id} className="flex items-center justify-between">
                    <div className="flex-1">
                      <Link
                        to={`/workflows/${workflow.id}`}
                        className="font-medium hover:underline"
                      >
                        {workflow.name}
                      </Link>
                      <p className="text-sm text-muted-foreground">
                        {workflow.node_count} nodes • Updated {new Date(workflow.updated_at).toLocaleDateString()}
                      </p>
                    </div>
                    <Badge variant={workflow.status === 'active' ? 'default' : 'secondary'}>
                      {workflow.status}
                    </Badge>
                  </div>
                ))
              )}
              {workflows.length > 0 && (
                <Button asChild variant="outline" className="w-full">
                  <Link to="/workflows">View all workflows</Link>
                </Button>
              )}
            </div>
          </CardContent>
        </Card>

        {/* Recent Executions */}
        <Card>
          <CardHeader>
            <CardTitle>Recent Executions</CardTitle>
            <CardDescription>
              Latest workflow execution activity
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {recentExecutions.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  <Play className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>No executions yet</p>
                  <p className="text-sm">Execute a workflow to see results here</p>
                </div>
              ) : (
                recentExecutions.slice(0, 5).map((execution) => (
                  <div key={execution.execution_id} className="flex items-center justify-between">
                    <div className="flex items-center gap-3 flex-1">
                      {getStatusIcon(execution.status)}
                      <div>
                        <Link
                          to={`/executions/${execution.execution_id}`}
                          className="font-medium hover:underline"
                        >
                          {execution.workflow_name}
                        </Link>
                        <p className="text-sm text-muted-foreground">
                          {new Date(execution.started_at).toLocaleString()}
                        </p>
                      </div>
                    </div>
                    {getStatusBadge(execution.status)}
                  </div>
                ))
              )}
              {recentExecutions.length > 0 && (
                <Button asChild variant="outline" className="w-full">
                  <Link to="/executions">View all executions</Link>
                </Button>
              )}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
};

export default Dashboard;
