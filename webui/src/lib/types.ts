// API Types for Papermake Server Integration

// Common types
export type RenderStatus = 'queued' | 'processing' | 'completed' | 'failed';
export type SortOrder = 'asc' | 'desc';
export type TimePeriod = 'hour' | 'day' | 'week' | 'month';
export type HealthStatus = 'healthy' | 'warning' | 'critical';

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
  id: string;
  name: string;
  latest_version: number;
  uses_24h: number;
  published_at: string;
  author: string;
}

export interface TemplateDetails {
  id: string;
  name: string;
  description: string | null;
  content: string;
  schema: any | null;
  version: number;
  author: string;
  published_at: string;
  uses_total: number;
  uses_24h: number;
}

export interface TemplateVersion {
  version: number;
  published_at: string;
  author: string;
  uses_total: number;
}

export interface CreateTemplateRequest {
  id: string;
  name: string;
  description?: string;
  content: string;
  schema?: any;
  author: string;
}

export interface UpdateTemplateRequest {
  name?: string;
  description?: string;
  content?: string;
  schema?: any;
  author: string;
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
export interface RenderJobSummary {
  id: string;
  template_id: string;
  template_version: number;
  status: RenderStatus;
  created_at: string;
  completed_at: string | null;
  rendering_latency: number | null;
  pdf_url: string | null;
}

export interface RenderJobDetails {
  id: string;
  template_id: string;
  template_version: number;
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
  template_id: string;
  template_version: number;
  data: any;
  options?: RenderOptions;
}

export interface CreateRenderResponse {
  id: string;
  status: RenderStatus;
  created_at: string;
  estimated_completion: string | null;
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

// Analytics Types
export interface DashboardMetrics {
  queue_depth: number;
  p90_latency_ms: number | null;
  total_renders_24h: number;
  success_rate_24h: number;
  recent_renders: RenderJobSummary[];
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

export interface TemplateAnalytics {
  template_id: string;
  template_name: string;
  total_versions: number;
  latest_version: number;
  usage_over_time: UsageDataPoint[];
  performance_metrics: TemplatePerformanceMetrics;
}

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

export interface SystemHealth {
  status: HealthStatus;
  uptime_seconds: number;
  queue_health: QueueHealth;
  storage_health: StorageHealth;
  last_updated: string;
}

export interface QueueHealth {
  status: HealthStatus;
  current_depth: number;
  max_depth_24h: number;
  processing_rate: number;
  avg_wait_time_ms: number;
}

export interface StorageHealth {
  status: HealthStatus;
  database_connected: boolean;
  s3_connected: boolean;
  database_response_time_ms: number | null;
  s3_response_time_ms: number | null;
}