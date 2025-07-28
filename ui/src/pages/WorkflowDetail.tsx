import React, { useEffect, useState, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { Textarea } from '../components/ui/textarea';
import { Separator } from '../components/ui/separator';
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from '../components/ui/resizable';
import {
  Save,
  Play,
  ArrowLeft,
  Code2,
  GitBranch,
} from 'lucide-react';
import MonacoEditor from '@monaco-editor/react';
import WorkflowCanvas from '../components/workflow/WorkflowCanvas';
import { useWorkflowStore, useAppStore } from '../stores';
import apiClient from '../services/api';
import { CreateWorkflowRequest } from '../types';
import { parseDsl, generateWorkflowDsl, validateWorkflowDsl } from '../utils/dslParser';

const WorkflowDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const [workflowName, setWorkflowName] = useState('');
  const [workflowDescription, setWorkflowDescription] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [dslContent, setDslContent] = useState('');
  const [dslError, setDslError] = useState<string | undefined>();
  const [isDslValid, setIsDslValid] = useState(false);

  const { isDarkMode } = useAppStore();
  const {
    currentWorkflow,
    setCurrentWorkflow,
    nodes,
    edges,
    setNodes,
    setEdges,
  } = useWorkflowStore();

  useEffect(() => {
    const loadWorkflow = async () => {
      if (id) {
        try {
          const response = await apiClient.getWorkflow(id);
          if (response.data) {
            setCurrentWorkflow(response.data);
            setWorkflowName(response.data.metadata.name);
            setWorkflowDescription(response.data.metadata.description || '');

            // Use the raw definition from the API response
            if (response.data.definition) {
              setDslContent(response.data.definition);
              validateDsl(response.data.definition);

              // Also parse the DSL to populate the visual editor
              const result = parseDsl(response.data.definition);
              if (result.isValid && result.nodes && result.edges) {
                setNodes(result.nodes);
                setEdges(result.edges);
              }
            }
          }
        } catch (error) {
          console.error('Failed to load workflow:', error);
          navigate('/workflows');
        }
      }
    };

    loadWorkflow();
  }, [id, setCurrentWorkflow, navigate]);

  const validateDsl = useCallback((dsl: string) => {
    const validation = validateWorkflowDsl(dsl);
    setIsDslValid(validation.isValid);
    setDslError(validation.errors.length > 0 ? validation.errors.join(', ') : undefined);
  }, []);

  const handleDslChange = useCallback((newDsl: string | undefined) => {
    if (newDsl !== undefined) {
      setDslContent(newDsl);
      validateDsl(newDsl);
    }
  }, [validateDsl]);

  const handleSyncToVisual = useCallback(() => {
    const result = parseDsl(dslContent);

    if (result.isValid && result.nodes && result.edges) {
      setNodes(result.nodes);
      setEdges(result.edges);

      if (result.workflow?.metadata) {
        setWorkflowName(result.workflow.metadata.name || workflowName);
        setWorkflowDescription(result.workflow.metadata.description || workflowDescription);
      }

      setDslError(undefined);
    } else {
      setDslError(result.error || 'Failed to parse DSL');
    }
  }, [dslContent, setNodes, setEdges, workflowName, workflowDescription]);

  const handleSyncFromVisual = useCallback(() => {
    if (nodes.length > 0) {
      const generatedDsl = generateWorkflowDsl(nodes, edges);
      setDslContent(generatedDsl);
      validateDsl(generatedDsl);
    }
  }, [nodes, edges, validateDsl]);

  const handleSave = async () => {
    if (!workflowName.trim()) {
      alert('Please enter a workflow name');
      return;
    }

    setIsSaving(true);

    try {
      let definitionYaml: string;

      if (isDslValid && dslContent) {
        definitionYaml = dslContent;
      } else {
        definitionYaml = generateWorkflowDsl(nodes, edges);
      }

      const workflowData: CreateWorkflowRequest = {
        name: workflowName,
        description: workflowDescription,
        definition: definitionYaml,
      };

      if (id) {
        await apiClient.updateWorkflow(id, workflowData);
      }
    } catch (error) {
      console.error('Failed to save workflow:', error);
      alert('Failed to save workflow');
    } finally {
      setIsSaving(false);
    }
  };

  const handleExecute = async () => {
    if (!currentWorkflow?.id) {
      alert('Please save the workflow before executing');
      return;
    }

    setIsExecuting(true);
    try {
      const response = await apiClient.executeWorkflow(currentWorkflow.id, { trigger_data: {} });
      if (response.data) {
        // Navigate to execution detail page
        navigate(`/executions/${response.data.execution_id}`);
      }
    } catch (error) {
      console.error('Failed to execute workflow:', error);
      alert('Failed to execute workflow');
    } finally {
      setIsExecuting(false);
    }
  };

  const monacoOptions = {
    minimap: { enabled: false },
    fontSize: 14,
    lineNumbers: 'on' as const,
    scrollBeyondLastLine: false,
    wordWrap: 'on' as const,
    automaticLayout: true,
    tabSize: 2,
    insertSpaces: true,
  };

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
                onClick={() => navigate('/workflows')}
              >
                <ArrowLeft className="h-4 w-4 mr-2" />
                Back to Workflows
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
                variant="outline"
                size="sm"
                onClick={handleSyncFromVisual}
                title="Sync from visual to DSL"
              >
                <Code2 className="h-4 w-4 mr-2" />
                DSL ← Visual
              </Button>

              <Button
                variant="outline"
                size="sm"
                onClick={handleSyncToVisual}
                title="Sync from DSL to visual"
                disabled={!isDslValid}
              >
                <GitBranch className="h-4 w-4 mr-2" />
                Visual ← DSL
              </Button>

              <Separator orientation="vertical" className="h-6" />

              <Button
                variant="outline"
                size="sm"
                onClick={handleSave}
                disabled={isSaving}
              >
                <Save className="h-4 w-4 mr-2" />
                {isSaving ? 'Saving...' : 'Save'}
              </Button>

              <Button
                size="sm"
                onClick={handleExecute}
                disabled={isExecuting || !currentWorkflow}
              >
                <Play className="h-4 w-4 mr-2" />
                {isExecuting ? 'Executing...' : 'Execute'}
              </Button>
            </div>
          </div>
        </div>
      </div>

      {/* Split View Content */}
      <div className="flex-1 overflow-hidden">
        <ResizablePanelGroup direction="horizontal">
          {/* DSL Editor Panel */}
          <ResizablePanel defaultSize={50} minSize={30}>
            <div className="h-full flex flex-col">
              <div className="px-4 py-2 border-b bg-muted/50">
                <div className="flex items-center justify-between">
                  <h3 className="text-sm font-medium flex items-center gap-2">
                    <Code2 className="h-4 w-4" />
                    DSL Editor
                  </h3>
                  {dslError && (
                    <span className="text-xs text-destructive">{dslError}</span>
                  )}
                  {isDslValid && !dslError && (
                    <span className="text-xs text-green-600">Valid</span>
                  )}
                </div>
              </div>
              <div className="flex-1">
                <MonacoEditor
                  height="100%"
                  language="yaml"
                  theme={isDarkMode ? "vs-dark" : "vs"}
                  value={dslContent}
                  onChange={handleDslChange}
                  options={monacoOptions}
                />
              </div>
            </div>
          </ResizablePanel>

          <ResizableHandle />

          {/* Visual Editor Panel */}
          <ResizablePanel defaultSize={50} minSize={30}>
            <div className="h-full flex flex-col">
              <div className="px-4 py-2 border-b bg-muted/50">
                <h3 className="text-sm font-medium flex items-center gap-2">
                  <GitBranch className="h-4 w-4" />
                  Visual Workflow
                </h3>
              </div>
              <div className="flex-1 relative">
                <WorkflowCanvas
                  onSave={handleSave}
                  readOnly={false}
                />
              </div>
            </div>
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    </div>
  );
};

export default WorkflowDetail;
