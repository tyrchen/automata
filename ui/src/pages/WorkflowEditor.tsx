import React, { useEffect, useState, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { Textarea } from '../components/ui/textarea';
import { Label } from '../components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card';
import { Separator } from '../components/ui/separator';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/tabs';
import {
  Save,
  Play,
  Eye,
  EyeOff,
  ArrowLeft,
  Code2,
  GitBranch
} from 'lucide-react';
import WorkflowCanvas from '../components/workflow/WorkflowCanvas';
import NodeLibrary from '../components/workflow/NodeLibrary';
import DslEditor from '../components/workflow/DslEditor';
import { useWorkflowStore, useAppStore } from '../stores';
import apiClient from '../services/api';
import { CreateWorkflowRequest } from '../types';
import { parseDsl, generateWorkflowDsl, validateWorkflowDsl } from '../utils/dslParser';

const WorkflowEditor: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const isNew = id === 'new';

  const [workflowName, setWorkflowName] = useState('');
  const [workflowDescription, setWorkflowDescription] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [activeTab, setActiveTab] = useState<'visual' | 'dsl'>('visual');
  const [dslContent, setDslContent] = useState('');
  const [dslError, setDslError] = useState<string | undefined>();
  const [isDslValid, setIsDslValid] = useState(false);

  const {
    currentWorkflow,
    setCurrentWorkflow,
    nodes,
    edges,
    availableNodes,
    setAvailableNodes,
    showNodeLibrary,
    setShowNodeLibrary,
    isSaving: storeIsSaving,
    setIsSaving: setStoreIsSaving,
    clearWorkflow,
    setNodes,
    setEdges,
  } = useWorkflowStore();

  useEffect(() => {
    const loadWorkflow = async () => {
      if (isNew) {
        clearWorkflow();
        setWorkflowName('');
        setWorkflowDescription('');
      } else if (id) {
        try {
          const response = await apiClient.getWorkflow(id);
          if (response.data) {
            setCurrentWorkflow(response.data);
            setWorkflowName(response.data.metadata.name);
            setWorkflowDescription(response.data.metadata.description || '');
          }
        } catch (error) {
          console.error('Failed to load workflow:', error);
          navigate('/workflows');
        }
      }
    };

    loadWorkflow();
  }, [id, isNew, setCurrentWorkflow, clearWorkflow, navigate]);

  useEffect(() => {
    const loadAvailableNodes = async () => {
      try {
        const response = await apiClient.getAvailableNodes();
        if (response.data) {
          setAvailableNodes(response.data.descriptions);
        }
      } catch (error) {
        console.error('Failed to load available nodes:', error);
      }
    };

    if (availableNodes.length === 0) {
      loadAvailableNodes();
    }
  }, [availableNodes.length, setAvailableNodes]);

  // Sync visual editor with DSL when switching tabs
  useEffect(() => {
    if (activeTab === 'dsl' && nodes.length > 0) {
      // Generate DSL from visual nodes
      const generatedDsl = generateWorkflowDsl(nodes, edges);
      setDslContent(generatedDsl);
      validateDsl(generatedDsl);
    }
  }, [activeTab, nodes, edges]);

  const validateDsl = useCallback((dsl: string) => {
    const validation = validateWorkflowDsl(dsl);
    setIsDslValid(validation.isValid);
    setDslError(validation.errors.length > 0 ? validation.errors.join(', ') : undefined);
  }, []);

  const handleDslChange = useCallback((newDsl: string) => {
    setDslContent(newDsl);
    validateDsl(newDsl);
  }, [validateDsl]);

  const handleParseDsl = useCallback(() => {
    const result = parseDsl(dslContent);

    if (result.isValid && result.nodes && result.edges) {
      // Update visual editor with parsed nodes
      setNodes(result.nodes);
      setEdges(result.edges);

      // Update workflow metadata if available
      if (result.workflow?.metadata) {
        setWorkflowName(result.workflow.metadata.name || workflowName);
        setWorkflowDescription(result.workflow.metadata.description || workflowDescription);
      }

      // Switch to visual tab to see the result
      setActiveTab('visual');
      setDslError(undefined);
    } else {
      setDslError(result.error || 'Failed to parse DSL');
    }
  }, [dslContent, setNodes, setEdges, workflowName, workflowDescription]);

  const handleSave = async () => {
    if (!workflowName.trim()) {
      alert('Please enter a workflow name');
      return;
    }

    setIsSaving(true);
    setStoreIsSaving(true);

    try {
      let definitionYaml: string;

      if (activeTab === 'dsl' && isDslValid && dslContent) {
        // Use DSL content directly
        definitionYaml = dslContent;
      } else {
        // Generate DSL from visual editor
        definitionYaml = generateWorkflowDsl(nodes, edges);

        // Override metadata with current form values
        const lines = definitionYaml.split('\n');
        const updatedLines = lines.map(line => {
          if (line.trim().startsWith('name:') && line.includes('metadata:')) {
            return `  name: "${workflowName}"`;
          }
          if (line.trim().startsWith('description:') && line.includes('metadata:')) {
            return `  description: "${workflowDescription}"`;
          }
          return line;
        });
        definitionYaml = updatedLines.join('\n');
      }

      const workflowData: CreateWorkflowRequest = {
        name: workflowName,
        description: workflowDescription,
        definition: definitionYaml,
      };

      if (isNew) {
        const response = await apiClient.createWorkflow(workflowData);
        if (response.data) {
          navigate(`/workflows/${response.data.id}/edit`);
        }
      } else if (id) {
        await apiClient.updateWorkflow(id, workflowData);
      }
    } catch (error) {
      console.error('Failed to save workflow:', error);
      alert('Failed to save workflow');
    } finally {
      setIsSaving(false);
      setStoreIsSaving(false);
    }
  };

  const handleExecute = async () => {
    if (!currentWorkflow && isNew) {
      alert('Please save the workflow before executing');
      return;
    }

    setIsExecuting(true);
    try {
      const workflowId = id === 'new' ? currentWorkflow?.id : id;
      if (workflowId) {
        await apiClient.executeWorkflow(workflowId, { trigger_data: {} });
        // Could show success message or redirect to executions
        alert('Workflow execution started!');
      }
    } catch (error) {
      console.error('Failed to execute workflow:', error);
      alert('Failed to execute workflow');
    } finally {
      setIsExecuting(false);
    }
  };

  return (
    <div className="h-full flex flex-col">
      {/* Editor Header */}
      <div className="border-b border-border bg-background/95 backdrop-blur">
        <div className="px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => navigate('/workflows')}
              >
                <ArrowLeft className="h-4 w-4 mr-2" />
                Back
              </Button>

              <div className="flex flex-col">
                <div className="flex items-center gap-2">
                  <Input
                    value={workflowName}
                    onChange={(e) => setWorkflowName(e.target.value)}
                    placeholder="Workflow name"
                    className="text-lg font-semibold border-none shadow-none p-0 h-auto focus-visible:ring-0"
                  />
                </div>
                <Textarea
                  value={workflowDescription}
                  onChange={(e) => setWorkflowDescription(e.target.value)}
                  placeholder="Description (optional)"
                  className="text-sm text-muted-foreground border-none shadow-none p-0 h-auto resize-none focus-visible:ring-0"
                  rows={1}
                />
              </div>
            </div>

            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setShowNodeLibrary(!showNodeLibrary)}
              >
                {showNodeLibrary ? (
                  <EyeOff className="h-4 w-4 mr-2" />
                ) : (
                  <Eye className="h-4 w-4 mr-2" />
                )}
                Node Library
              </Button>

              <Separator orientation="vertical" className="h-6" />

              <Button
                variant="outline"
                size="sm"
                onClick={handleSave}
                disabled={isSaving || storeIsSaving}
              >
                <Save className="h-4 w-4 mr-2" />
                {isSaving || storeIsSaving ? 'Saving...' : 'Save'}
              </Button>

              <Button
                size="sm"
                onClick={handleExecute}
                disabled={isExecuting || (!currentWorkflow && isNew)}
              >
                <Play className="h-4 w-4 mr-2" />
                {isExecuting ? 'Executing...' : 'Execute'}
              </Button>
            </div>
          </div>
        </div>
      </div>

      {/* Editor Content */}
      <div className="flex-1 flex overflow-hidden">
        <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as 'visual' | 'dsl')} className="flex-1 flex flex-col">
          <div className="px-6 py-2 border-b">
            <TabsList>
              <TabsTrigger value="visual" className="gap-2">
                <GitBranch className="h-4 w-4" />
                Visual Editor
              </TabsTrigger>
              <TabsTrigger value="dsl" className="gap-2">
                <Code2 className="h-4 w-4" />
                DSL Editor
              </TabsTrigger>
            </TabsList>
          </div>

          <TabsContent value="visual" className="flex-1 flex overflow-hidden m-0">
            {/* Node Library */}
            {showNodeLibrary && (
              <NodeLibrary onClose={() => setShowNodeLibrary(false)} />
            )}

            {/* Canvas */}
            <div className="flex-1 relative">
              <WorkflowCanvas
                onSave={handleSave}
                onExecute={handleExecute}
                readOnly={false}
              />
            </div>
          </TabsContent>

          <TabsContent value="dsl" className="flex-1 overflow-auto m-0 p-6">
            <DslEditor
              value={dslContent}
              onChange={handleDslChange}
              onParse={handleParseDsl}
              onExecute={handleExecute}
              error={dslError}
              isValid={isDslValid}
            />
          </TabsContent>
        </Tabs>

        {/* Properties Panel (for selected node) */}
        <div className="w-80 border-l border-border bg-background">
          <div className="p-4 h-full overflow-auto">
            <h3 className="text-lg font-semibold mb-4">Properties</h3>

            {/* Workflow Properties */}
            <Card className="mb-4">
              <CardHeader>
                <CardTitle className="text-base">Workflow</CardTitle>
                <CardDescription>Basic workflow information</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <Label htmlFor="workflow-name">Name</Label>
                  <Input
                    id="workflow-name"
                    value={workflowName}
                    onChange={(e) => setWorkflowName(e.target.value)}
                    placeholder="Enter workflow name"
                  />
                </div>
                <div>
                  <Label htmlFor="workflow-description">Description</Label>
                  <Textarea
                    id="workflow-description"
                    value={workflowDescription}
                    onChange={(e) => setWorkflowDescription(e.target.value)}
                    placeholder="Enter workflow description"
                    rows={3}
                  />
                </div>
              </CardContent>
            </Card>

            {/* Canvas Stats */}
            <Card>
              <CardHeader>
                <CardTitle className="text-base">Canvas Stats</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-2 text-sm">
                  <div className="flex justify-between">
                    <span>Nodes:</span>
                    <span>{nodes.length}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Connections:</span>
                    <span>{edges.length}</span>
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* Node-specific properties would go here when a node is selected */}
          </div>
        </div>
      </div>
    </div>
  );
};

export default WorkflowEditor;
