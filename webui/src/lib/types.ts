// API Types for Papermake Server Integration

// Common types
export type RenderStatus = "queued" | "processing" | "completed" | "failed";
export type SortOrder = "asc" | "desc";
export type TimePeriod = "hour" | "day" | "week" | "month";
// HealthStatus removed - server analytics not yet implemented

// Pagination
export interface PaginationQuery {
  limit?: number;
  offset?: number;
}

export interface PaginationInfo {
  limit: number;
  offset: number;
  total: number | null;
  has_more: boolean;
}

export interface PaginatedResponse<T> {
  data: T[];
  pagination: PaginationInfo;
}

export interface ApiResponse<T> {
  data: T;
  message?: string;
}

export interface SearchQuery extends PaginationQuery {
  search?: string;
  sort_by?: string;
  sort_order?: SortOrder;
}

// Template Types
export interface TemplateSummary {
  name: string;
  latest_version: string;
  uses_24h: number;
  published_at: string;
  author: string;
}

export interface TemplateInfo {
  name: string;
  namespace: string | null;
  tags: string[];
  latest_manifest_hash: string | null;
  metadata: {
    name: string;
    author: string;
  };
}

export interface TemplateMetadataResponse {
  name: string;
  namespace: string | null;
  tag: string;
  tags: string[];
  manifest_hash: string;
  metadata: TemplateMetadata;
  reference: string;
}

export interface TemplateDetails {
  name: string;
  description: string | null;
  content: string;
  schema: any | null;
  version: string;
  author: string;
  published_at: string;
  uses_total: number;
  uses_24h: number;
}

export interface TemplateVersion {
  version: string; // Changed from number to string
  published_at: string;
  author: string;
  uses_total: number;
}

export interface PublishTemplateRequest {
  main_typ: string;
  metadata: TemplateMetadata;
  schema?: any;
}

export interface TemplateMetadata {
  name: string;
  author: string;
}

export interface PublishResponse {
  message: string;
  manifest_hash: string;
  reference: string;
}

export interface TemplatePreviewRequest {
  content: string;
  data: any;
  schema?: any;
}

export interface TemplateValidationRequest {
  content: string;
  schema?: any;
  data?: any;
}

export interface ValidationError {
  message: string;
  line: number | null;
  column: number | null;
}

export interface ValidationWarning {
  message: string;
  line: number | null;
  column: number | null;
}

export interface TemplateValidationResponse {
  valid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
}

// Render Types
export interface RenderRecord {
  render_id: string;
  timestamp: string;
  template_ref: string;
  template_name: string;
  template_tag: string;
  manifest_hash: string;
  data_hash: string;
  pdf_hash: string;
  success: boolean;
  duration_ms: number;
  pdf_size_bytes: number;
  error: string | null;
}

export interface RenderJobDetails {
  id: string;
  template_ref: string;
  data: any;
  data_hash: string;
  status: RenderStatus;
  created_at: string;
  completed_at: string | null;
  rendering_latency: number | null;
  pdf_url: string | null;
  error_message: string | null;
}

export interface RenderOptions {
  paper_size?: string;
  compress?: boolean;
  priority?: number;
}

export interface CreateRenderRequest {
  data: any;
}

export interface RenderResponse {
  render_id: string;
  pdf_hash: string;
  duration_ms: number;
}

export interface BatchRenderRequest {
  requests: CreateRenderRequest[];
}

export interface BatchRenderResponse {
  batch_id: string;
  render_jobs: CreateRenderResponse[];
  total_jobs: number;
}

export interface RenderJobQuery extends PaginationQuery {
  template_id?: string;
  status?: RenderStatus;
  date_from?: string;
  date_to?: string;
}

export interface RenderJobUpdate {
  job_id: string;
  status: RenderStatus;
  progress: number | null;
  message: string | null;
  completed_at: string | null;
  pdf_url: string | null;
}

// Analytics Types (server not yet implemented - placeholder)
export interface DashboardMetrics {
  queue_depth: number;
  p90_latency_ms: number | null;
  total_renders_24h: number;
  success_rate_24h: number;
  recent_renders: RenderRecord[];
  popular_templates: TemplateUsage[];
  new_templates: TemplateSummary[];
}

export interface TemplateUsage {
  template_id: string;
  template_name: string;
  version: number;
  uses_24h: number;
  uses_7d: number;
  uses_30d: number;
  uses_total: number;
  published_at: string;
  avg_render_time_ms: number | null;
}

export interface PerformanceMetrics {
  period: TimePeriod;
  data_points: PerformanceDataPoint[];
  summary: PerformanceSummary;
}

export interface PerformanceDataPoint {
  timestamp: string;
  total_renders: number;
  successful_renders: number;
  failed_renders: number;
  avg_latency_ms: number | null;
  p90_latency_ms: number | null;
  queue_depth: number | null;
}

export interface PerformanceSummary {
  total_renders: number;
  success_rate: number;
  avg_latency_ms: number;
  p50_latency_ms: number;
  p90_latency_ms: number;
  p99_latency_ms: number;
}

export interface AnalyticsQuery {
  period?: TimePeriod;
  date_from?: string;
  date_to?: string;
  template_id?: string;
  limit?: number;
}

// TemplateAnalytics removed - server analytics not yet implemented

export interface UsageDataPoint {
  timestamp: string;
  renders: number;
  unique_data_hashes: number;
  avg_render_time_ms: number | null;
}

export interface TemplatePerformanceMetrics {
  total_renders: number;
  successful_renders: number;
  failed_renders: number;
  success_rate: number;
  avg_render_time_ms: number;
  fastest_render_ms: number | null;
  slowest_render_ms: number | null;
  cache_hit_rate: number;
}

// SystemHealth removed - server analytics not yet implemented

// QueueHealth and StorageHealth removed - server analytics not yet implemented

// Mock data generation functions (kept for fallback scenarios)
export function generateMockDashboardMetrics(): DashboardMetrics {
  return {
    queue_depth: 0,
    p90_latency_ms: null,
    total_renders_24h: 0,
    success_rate_24h: 0,
    recent_renders: [],
    popular_templates: [],
    new_templates: [],
  };
}

export function generateMockTemplateUsage(count: number): TemplateUsage[] {
  return [];
}

export function generateMockTemplateSummaries(
  count: number,
): TemplateSummary[] {
  return [];
}
