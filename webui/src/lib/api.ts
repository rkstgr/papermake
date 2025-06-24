// API Client for Papermake Server
import {
  DashboardMetrics,
  TemplateSummary,
  TemplateDetails,
  TemplateVersion,
  CreateTemplateRequest,
  TemplatePreviewRequest,
  TemplateValidationRequest,
  TemplateValidationResponse,
  RenderJobSummary,
  RenderJobDetails,
  CreateRenderRequest,
  CreateRenderResponse,
  BatchRenderRequest,
  BatchRenderResponse,
  RenderJobQuery,
  TemplateUsage,
  TemplateAnalytics,
  SystemHealth,
  PaginatedResponse,
  ApiResponse,
  SearchQuery,
  AnalyticsQuery
} from './types';

const API_BASE = process.env.NEXT_PUBLIC_API_BASE_URL || '';

class ApiError extends Error {
  constructor(
    public status: number,
    public statusText: string,
    message: string
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

async function apiRequest<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const url = `${API_BASE}${endpoint}`;
  
  const response = await fetch(url, {
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
    ...options,
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new ApiError(
      response.status,
      response.statusText,
      errorText || `Request failed: ${response.status} ${response.statusText}`
    );
  }

  // Handle PDF responses
  if (response.headers.get('content-type')?.includes('application/pdf')) {
    return response.blob() as unknown as T;
  }

  const data = await response.json();
  
  // Handle ApiResponse wrapper - if the response has 'data' field, extract it
  if (data && typeof data === 'object' && 'data' in data) {
    return data.data as T;
  }
  
  return data;
}

// Helper function to build query string
function buildQueryString(params: Record<string, any>): string {
  const searchParams = new URLSearchParams();
  
  Object.entries(params).forEach(([key, value]) => {
    if (value !== undefined && value !== null) {
      searchParams.append(key, String(value));
    }
  });
  
  const queryString = searchParams.toString();
  return queryString ? `?${queryString}` : '';
}

// Template API
export const templateApi = {
  // List templates
  async list(query: SearchQuery = {}): Promise<PaginatedResponse<TemplateSummary>> {
    const queryString = buildQueryString(query);
    return apiRequest<PaginatedResponse<TemplateSummary>>(`/api/templates${queryString}`);
  },

  // Get template by ID (latest version)
  async get(templateId: string): Promise<TemplateDetails> {
    return apiRequest<TemplateDetails>(`/api/templates/${templateId}`);
  },

  // Get template versions
  async getVersions(templateId: string): Promise<number[]> {
    return apiRequest<number[]>(`/api/templates/${templateId}/versions`);
  },

  // Get specific template version
  async getVersion(templateId: string, version: number): Promise<TemplateDetails> {
    return apiRequest<TemplateDetails>(`/api/templates/${templateId}/versions/${version}`);
  },

  // Create template
  async create(template: CreateTemplateRequest): Promise<TemplateDetails> {
    return apiRequest<TemplateDetails>('/api/templates', {
      method: 'POST',
      body: JSON.stringify(template),
    });
  },

  // Preview template (returns PDF blob directly)
  async preview(request: TemplatePreviewRequest): Promise<Blob> {
    return apiRequest<Blob>('/api/templates/preview', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  },

  // Validate template
  async validate(request: TemplateValidationRequest): Promise<TemplateValidationResponse> {
    return apiRequest<TemplateValidationResponse>('/api/templates/validate', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  },
};

// Render API
export const renderApi = {
  // List render jobs
  async list(query: RenderJobQuery = {}): Promise<PaginatedResponse<RenderJobSummary>> {
    const queryString = buildQueryString(query);
    return apiRequest<PaginatedResponse<RenderJobSummary>>(`/api/renders${queryString}`);
  },

  // Get render job by ID
  async get(renderId: string): Promise<RenderJobDetails> {
    return apiRequest<RenderJobDetails>(`/api/renders/${renderId}`);
  },

  // Create render job
  async create(request: CreateRenderRequest): Promise<CreateRenderResponse> {
    return apiRequest<CreateRenderResponse>('/api/renders', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  },

  // Create batch render jobs
  async createBatch(request: BatchRenderRequest): Promise<BatchRenderResponse> {
    return apiRequest<BatchRenderResponse>('/api/renders/batch', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  },

  // Download PDF
  async downloadPdf(renderId: string): Promise<Blob> {
    return apiRequest<Blob>(`/api/renders/${renderId}/pdf`);
  },

  // Retry render job
  async retry(renderId: string): Promise<CreateRenderResponse> {
    return apiRequest<CreateRenderResponse>(`/api/renders/${renderId}/retry`, {
      method: 'POST',
    });
  },
};

// Analytics API
export const analyticsApi = {
  // Get dashboard metrics
  async getDashboard(): Promise<DashboardMetrics> {
    return apiRequest<DashboardMetrics>('/api/analytics/dashboard');
  },

  // Get template usage statistics
  async getTemplateUsage(query: AnalyticsQuery = {}): Promise<TemplateUsage[]> {
    const queryString = buildQueryString(query);
    return apiRequest<TemplateUsage[]>(`/api/analytics/templates/usage${queryString}`);
  },

  // Get template analytics
  async getTemplateAnalytics(templateId: string): Promise<TemplateAnalytics> {
    return apiRequest<TemplateAnalytics>(`/api/analytics/templates/${templateId}/analytics`);
  },

  // Get system health
  async getHealth(): Promise<SystemHealth> {
    return apiRequest<SystemHealth>('/api/analytics/health');
  },
};

// Health check
export const healthApi = {
  async check(): Promise<ApiResponse<any>> {
    return apiRequest<ApiResponse<any>>('/health');
  },
};

// Export convenience functions
export { ApiError };

// Helper function to format render latency
export function formatLatency(latencyMs: number | null): string {
  if (latencyMs === null) return 'N/A';
  if (latencyMs < 1000) return `${Math.round(latencyMs)}ms`;
  return `${(latencyMs / 1000).toFixed(1)}s`;
}

// Helper function to format file size
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

// Helper function to format dates
export function formatDate(dateString: string): string {
  return new Date(dateString).toLocaleString();
}

// Helper function to format relative time
export function formatRelativeTime(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  
  const diffSeconds = Math.floor(diffMs / 1000);
  const diffMinutes = Math.floor(diffSeconds / 60);
  const diffHours = Math.floor(diffMinutes / 60);
  const diffDays = Math.floor(diffHours / 24);
  
  if (diffSeconds < 60) return `${diffSeconds}s ago`;
  if (diffMinutes < 60) return `${diffMinutes}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  
  return formatDate(dateString);
}