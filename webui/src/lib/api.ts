// API Client for Papermake Server
import {
  TemplateSummary,
  TemplateDetails,
  TemplateInfo,
  TemplateMetadataResponse,
  PublishTemplateRequest,
  PublishResponse,
  RenderRecord,
  RenderJobDetails,
  CreateRenderRequest,
  RenderResponse,
  RenderJobQuery,
  TemplateUsage,
  PaginatedResponse,
  ApiResponse,
  SearchQuery,
} from "./types";

const API_BASE =
  process.env.NEXT_PUBLIC_API_BASE_URL || "http://localhost:3000";

class ApiError extends Error {
  constructor(
    public status: number,
    public statusText: string,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

async function apiRequest<T>(
  endpoint: string,
  options: RequestInit = {},
): Promise<T> {
  const url = `${API_BASE}${endpoint}`;

  const response = await fetch(url, {
    headers: {
      "Content-Type": "application/json",
      ...options.headers,
    },
    ...options,
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new ApiError(
      response.status,
      response.statusText,
      errorText || `Request failed: ${response.status} ${response.statusText}`,
    );
  }

  // Handle PDF responses
  if (response.headers.get("content-type")?.includes("application/pdf")) {
    return response.blob() as unknown as T;
  }

  const data = await response.json();

  return data;
}

// Helper function to build query string
function buildQueryString(params: Record<string, any>): string {
  const searchParams = new URLSearchParams();

  Object.entries(params).forEach(([key, value]) => {
    if (value !== undefined && value !== null) {
      // Convert numbers to strings properly for URL encoding
      searchParams.append(key, String(value));
    }
  });

  const queryString = searchParams.toString();
  return queryString ? `?${queryString}` : "";
}

// Template API
export const templateApi = {
  // List templates
  async list(
    query: SearchQuery = {},
  ): Promise<PaginatedResponse<TemplateInfo>> {
    const queryString = buildQueryString(query);
    return apiRequest<PaginatedResponse<TemplateInfo>>(`/api/templates`);
  },

  // Get template metadata by reference
  async get(reference: string): Promise<TemplateMetadataResponse> {
    return apiRequest<TemplateMetadataResponse>(`/api/templates/${reference}`);
  },

  // Get template tags by name
  async getTags(templateName: string): Promise<string[]> {
    return apiRequest<string[]>(`/api/templates/${templateName}/tags`);
  },

  // Publish template (multipart form)
  async publish(
    templateName: string,
    formData: FormData,
    tag: string = "latest",
  ): Promise<PublishResponse> {
    const url = `${API_BASE}/api/templates/${templateName}/publish?tag=${tag}`;

    const response = await fetch(url, {
      method: "POST",
      body: formData,
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new ApiError(
        response.status,
        response.statusText,
        errorText ||
          `Request failed: ${response.status} ${response.statusText}`,
      );
    }

    const data = await response.json();
    return data && typeof data === "object" && "data" in data
      ? data.data
      : data;
  },

  // Publish template (JSON)
  async publishSimple(
    templateName: string,
    request: PublishTemplateRequest,
    tag: string = "latest",
  ): Promise<PublishResponse> {
    return apiRequest<PublishResponse>(
      `/api/templates/${templateName}/publish-simple?tag=${tag}`,
      {
        method: "POST",
        body: JSON.stringify(request),
      },
    );
  },
};

// Render API
export const renderApi = {
  // List recent renders
  async list(
    query: SearchQuery = {},
  ): Promise<PaginatedResponse<RenderRecord>> {
    const queryString = buildQueryString(query);
    return apiRequest<PaginatedResponse<RenderRecord>>(
      `/api/renders${queryString}`,
    );
  },

  // Render template
  async render(
    reference: string,
    request: CreateRenderRequest,
  ): Promise<RenderResponse> {
    return apiRequest<RenderResponse>(`/api/render/${reference}`, {
      method: "POST",
      body: JSON.stringify(request),
    });
  },

  // Download PDF by render ID
  async downloadPdf(renderId: string): Promise<Blob> {
    return apiRequest<Blob>(`/api/renders/${renderId}/pdf`);
  },
};

// Analytics API (not implemented yet)
export const analyticsApi = {
  // Placeholder - server analytics endpoints not yet implemented
  // These will be added when server implements analytics routes
};

// Health check
export const healthApi = {
  async check(): Promise<any> {
    return apiRequest<any>("/health");
  },
};

// Export convenience functions
export { ApiError };

// Helper function to format render latency
export function formatLatency(latencyMs: number | null): string {
  if (latencyMs === null) return "N/A";
  if (latencyMs < 1000) return `${Math.round(latencyMs)}ms`;
  return `${(latencyMs / 1000).toFixed(1)}s`;
}

// Helper function to format file size
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
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
