/**
 * API client for Automata workflow engine
 */

import {
  ApiResponse,
  ApiClientConfig,
  RequestConfig,
  CreateWorkflowRequest,
  CreateWorkflowResponse,
  GetWorkflowResponse,
  ExecuteWorkflowRequest,
  ExecuteWorkflowResponse,
  GetExecutionResponse,
  ListNodesResponse,
  WorkflowListItem,
  ExecutionListItem,
  PaginationParams,
  ListWorkflowsResponse,
  ListExecutionsResponse,
} from '../types';

class ApiClient {
  private baseUrl: string;
  private timeout: number;
  private defaultHeaders: Record<string, string>;

  constructor(config: ApiClientConfig) {
    this.baseUrl = config.baseUrl;
    this.timeout = config.timeout || 30000;
    this.defaultHeaders = {
      'Content-Type': 'application/json',
      ...config.headers,
    };
  }

  private async request<T = any>(config: RequestConfig): Promise<ApiResponse<T>> {
    const url = `${this.baseUrl}${config.url}`;
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await fetch(url, {
        method: config.method,
        headers: {
          ...this.defaultHeaders,
          ...config.headers,
        },
        body: config.data ? JSON.stringify(config.data) : undefined,
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      const contentType = response.headers.get('content-type');
      const hasJson = contentType?.includes('application/json');
      const data = hasJson ? await response.json() : await response.text();

      if (!response.ok) {
        return {
          data: undefined,
          error: data.message || `HTTP ${response.status}: ${response.statusText}`,
          status: response.status,
        };
      }

      return {
        data,
        error: undefined,
        status: response.status,
      };
    } catch (error) {
      clearTimeout(timeoutId);

      if (error instanceof Error) {
        if (error.name === 'AbortError') {
          return {
            data: undefined,
            error: 'Request timeout',
            status: 408,
          };
        }

        return {
          data: undefined,
          error: error.message,
          status: 0,
        };
      }

      return {
        data: undefined,
        error: 'Unknown error occurred',
        status: 0,
      };
    }
  }

  // Workflow endpoints
  async getWorkflows(params?: PaginationParams): Promise<ApiResponse<ListWorkflowsResponse>> {
    const queryParams = new URLSearchParams();
    if (params?.page) queryParams.append('page', params.page.toString());
    if (params?.limit) queryParams.append('limit', params.limit.toString());
    if (params?.sort) queryParams.append('sort', params.sort);
    if (params?.order) queryParams.append('order', params.order);

    const query = queryParams.toString();
    const url = `/api/v1/workflows${query ? `?${query}` : ''}`;

    return this.request<ListWorkflowsResponse>({
      method: 'GET',
      url,
    });
  }

  async getWorkflow(id: string): Promise<ApiResponse<GetWorkflowResponse>> {
    return this.request<GetWorkflowResponse>({
      method: 'GET',
      url: `/api/v1/workflows/${id}`,
    });
  }

  async createWorkflow(data: CreateWorkflowRequest): Promise<ApiResponse<CreateWorkflowResponse>> {
    return this.request<CreateWorkflowResponse>({
      method: 'POST',
      url: '/api/v1/workflows',
      data,
    });
  }

  async updateWorkflow(id: string, data: Partial<CreateWorkflowRequest>): Promise<ApiResponse<void>> {
    return this.request<void>({
      method: 'PUT',
      url: `/api/v1/workflows/${id}`,
      data,
    });
  }

  async deleteWorkflow(id: string): Promise<ApiResponse<void>> {
    return this.request<void>({
      method: 'DELETE',
      url: `/api/v1/workflows/${id}`,
    });
  }

  async executeWorkflow(id: string, data: ExecuteWorkflowRequest): Promise<ApiResponse<ExecuteWorkflowResponse>> {
    return this.request<ExecuteWorkflowResponse>({
      method: 'POST',
      url: `/api/v1/workflows/${id}/execute`,
      data,
    });
  }

  // Execution endpoints
  async getExecutions(params?: PaginationParams & { workflowId?: string }): Promise<ApiResponse<ListExecutionsResponse>> {
    const queryParams = new URLSearchParams();
    if (params?.page) queryParams.append('page', params.page.toString());
    if (params?.limit) queryParams.append('limit', params.limit.toString());
    if (params?.sort) queryParams.append('sort', params.sort);
    if (params?.order) queryParams.append('order', params.order);
    if (params?.workflowId) queryParams.append('workflow_id', params.workflowId);

    const query = queryParams.toString();
    const url = `/api/v1/executions${query ? `?${query}` : ''}`;

    return this.request<ListExecutionsResponse>({
      method: 'GET',
      url,
    });
  }

  async getExecution(id: string): Promise<ApiResponse<GetExecutionResponse>> {
    return this.request<GetExecutionResponse>({
      method: 'GET',
      url: `/api/v1/executions/${id}`,
    });
  }

  async cancelExecution(id: string): Promise<ApiResponse<void>> {
    return this.request<void>({
      method: 'POST',
      url: `/api/v1/executions/${id}/cancel`,
    });
  }

  // Node endpoints
  async getAvailableNodes(): Promise<ApiResponse<ListNodesResponse>> {
    return this.request<ListNodesResponse>({
      method: 'GET',
      url: '/api/v1/nodes',
    });
  }

  // Health check
  async health(): Promise<ApiResponse<{ status: string; timestamp: string }>> {
    return this.request<{ status: string; timestamp: string }>({
      method: 'GET',
      url: '/health',
    });
  }
}

// Create singleton instance
const apiClient = new ApiClient({
  baseUrl: import.meta.env.VITE_API_BASE_URL || 'http://localhost:8080',
  timeout: 30000,
});

export default apiClient;
