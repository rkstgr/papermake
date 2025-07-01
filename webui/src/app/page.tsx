"use client";

import React, { useEffect, useState } from "react";
import { DefaultPageLayout } from "@/ui/layouts/DefaultPageLayout";
import { Badge } from "@/ui/components/Badge";
import { FeatherArrowUp } from "@subframe/core";
import { FeatherArrowDown } from "@subframe/core";
import { FeatherPlus } from "@subframe/core";
import { TextField } from "@/ui/components/TextField";
import { FeatherSearch } from "@subframe/core";
import { Table } from "@/ui/components/Table";
import { FeatherCheckCircle } from "@subframe/core";
import { Avatar } from "@/ui/components/Avatar";
import { IconButton } from "@/ui/components/IconButton";
import { FeatherDownload } from "@subframe/core";
import { FeatherClock } from "@subframe/core";
import { Button } from "@/ui/components/Button";
import { Tabs } from "@/ui/components/Tabs";
import { FeatherBarChart2 } from "@subframe/core";
import {
  renderApi,
  templateApi,
  formatLatency,
  formatRelativeTime,
  formatFileSize,
} from "@/lib/api";
import { RenderRecord, TemplateInfo } from "@/lib/types";
import NewTemplateModal from "@/components/NewTemplateModal";

function RenderInsightsHub() {
  const [recentRenders, setRecentRenders] = useState<RenderRecord[]>([]);
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [totalRenders24h, setTotalRenders24h] = useState(0);
  const [p90Latency, setP90Latency] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [newTemplateModalOpen, setNewTemplateModalOpen] = useState(false);

  useEffect(() => {
    const fetchDashboardData = async () => {
      try {
        setLoading(true);

        // Fetch real render data from the API
        try {
          const rendersResponse = await renderApi.list({ limit: 50 });
          const renderData = rendersResponse.data;
          console.log(renderData);
          setRecentRenders(renderData);

          // Calculate metrics from render data
          const last24h = Date.now() - 24 * 60 * 60 * 1000;
          const renders24h = renderData.filter(
            (r) => new Date(r.timestamp).getTime() > last24h,
          );
          setTotalRenders24h(renders24h.length);

          // Calculate P90 latency from recent renders
          if (renderData.length > 0) {
            const sortedDurations = renderData
              .map((r) => r.duration_ms)
              .sort((a, b) => a - b);
            const p90Index = Math.floor(sortedDurations.length * 0.9);
            setP90Latency(sortedDurations[p90Index] || null);
          }
        } catch (renderErr) {
          console.warn("Failed to fetch render data:", renderErr);
          setRecentRenders([]);
          setTotalRenders24h(0);
          setP90Latency(null);
        }

        // Fetch real template data from the API
        try {
          const recentTemplates = await templateApi.list({ limit: 10 });
          setTemplates(recentTemplates.data);
        } catch (templateErr) {
          console.warn("Failed to fetch template data:", templateErr);
          setTemplates([]);
        }

        setError(null);
      } catch (err) {
        console.error("Failed to fetch dashboard data:", err);
        setError("Failed to load dashboard data");
      } finally {
        setLoading(false);
      }
    };

    fetchDashboardData();
  }, []);

  if (loading) {
    return (
      <DefaultPageLayout>
        <div className="container max-w-none flex h-full w-full flex-col items-center justify-center">
          <div className="text-lg">Loading dashboard...</div>
        </div>
      </DefaultPageLayout>
    );
  }

  if (error) {
    return (
      <DefaultPageLayout>
        <div className="container max-w-none flex h-full w-full flex-col items-center justify-center">
          <div className="text-lg text-red-600">{error}</div>
        </div>
      </DefaultPageLayout>
    );
  }

  return (
    <DefaultPageLayout>
      <div className="container max-w-none flex h-full w-full flex-col items-start gap-8 bg-default-background py-12">
        <div className="flex w-full flex-col items-start gap-1">
          <span className="w-full text-heading-2 font-heading-2 text-default-font">
            PDF Generation Dashboard
          </span>
          <span className="w-full text-body-bold font-body-bold text-subtext-color">
            Last updated just now
          </span>
        </div>
        <div className="flex w-full flex-wrap items-start gap-4">
          <div className="flex grow shrink-0 basis-0 flex-col items-start gap-4 rounded-md border border-solid border-neutral-border bg-default-background px-4 py-4">
            <span className="line-clamp-1 w-full text-caption-bold font-caption-bold text-subtext-color">
              Total Renders
            </span>
            <div className="flex w-full flex-col items-start gap-2">
              <span className="text-heading-2 font-heading-2 text-default-font">
                {totalRenders24h.toLocaleString()}
              </span>
              <Badge variant="success" icon={<FeatherArrowUp />}>
                24h total
              </Badge>
            </div>
          </div>
          <div className="flex grow shrink-0 basis-0 flex-col items-start gap-4 rounded-md border border-solid border-neutral-border bg-default-background px-4 py-4">
            <span className="line-clamp-1 w-full text-caption-bold font-caption-bold text-subtext-color">
              Average Render Time
            </span>
            <div className="flex w-full flex-col items-start gap-2">
              <span className="text-heading-2 font-heading-2 text-default-font">
                {p90Latency ? formatLatency(p90Latency) : "N/A"}
              </span>
              <Badge variant="neutral">P90 latency</Badge>
            </div>
          </div>
          <div className="flex grow shrink-0 basis-0 flex-col items-start gap-4 rounded-md border border-solid border-neutral-border bg-default-background px-4 py-4">
            <span className="line-clamp-1 w-full text-caption-bold font-caption-bold text-subtext-color">
              Active Templates
            </span>
            <div className="flex w-full flex-col items-start gap-2">
              <span className="text-heading-2 font-heading-2 text-default-font">
                {templates.length}
              </span>
              <Badge variant="neutral" icon={<FeatherPlus />}>
                0 new
              </Badge>
            </div>
          </div>
        </div>
        <div className="flex w-full flex-col items-start gap-6 rounded-md border border-solid border-neutral-border bg-default-background px-6 py-6 shadow-sm">
          <div className="flex w-full items-center gap-2">
            <span className="grow shrink-0 basis-0 text-heading-3 font-heading-3 text-default-font">
              Render Volume
            </span>
          </div>
          <div className="flex h-64 w-full flex-none items-center justify-center border border-dashed border-neutral-border bg-neutral-50 rounded-md">
            <span className="text-body font-body text-subtext-color">
              Analytics charts will be implemented soon
            </span>
          </div>
        </div>
        <div className="flex w-full flex-col items-start gap-6">
          <div className="flex w-full items-center justify-between">
            <span className="text-heading-3 font-heading-3 text-default-font">
              Recent Renders
            </span>
            <TextField
              variant="filled"
              label=""
              helpText=""
              icon={<FeatherSearch />}
            >
              <TextField.Input
                placeholder="Search renders..."
                value=""
                onChange={(event: React.ChangeEvent<HTMLInputElement>) => {}}
              />
            </TextField>
          </div>
          <Table
            header={
              <Table.HeaderRow>
                <Table.HeaderCell>Render ID</Table.HeaderCell>
                <Table.HeaderCell>Template</Table.HeaderCell>
                <Table.HeaderCell>Status</Table.HeaderCell>
                <Table.HeaderCell>Render Time</Table.HeaderCell>
                <Table.HeaderCell>Completed</Table.HeaderCell>
                <Table.HeaderCell />
              </Table.HeaderRow>
            }
          >
            {recentRenders.length === 0 ? (
              <Table.Row>
                <Table.Cell colSpan={6}>
                  <div className="flex w-full items-center justify-center py-8">
                    <span className="text-body font-body text-subtext-color">
                      No recent renders found
                    </span>
                  </div>
                </Table.Cell>
              </Table.Row>
            ) : (
              recentRenders.map((render) => {
                const getStatusBadge = (success: boolean) => {
                  if (success) {
                    return (
                      <Badge variant="success" icon={<FeatherCheckCircle />}>
                        Success
                      </Badge>
                    );
                  } else {
                    return <Badge variant="error">Failed</Badge>;
                  }
                };

                const handleDownload = async () => {
                  if (render.success && render.pdf_hash) {
                    try {
                      const blob = await renderApi.downloadPdf(
                        render.render_id,
                      );
                      const url = URL.createObjectURL(blob);
                      const a = document.createElement("a");
                      a.href = url;
                      a.download = `${render.template_ref}-${render.render_id}.pdf`;
                      document.body.appendChild(a);
                      a.click();
                      document.body.removeChild(a);
                      URL.revokeObjectURL(url);
                    } catch (error) {
                      console.error("Failed to download PDF:", error);
                    }
                  }
                };

                const handleCopyId = async () => {
                  try {
                    await navigator.clipboard.writeText(render.render_id);
                  } catch (error) {
                    console.error("Failed to copy ID:", error);
                  }
                };

                // Truncate render ID to first 8 characters
                const truncatedId = render.render_id.substring(24);

                return (
                  <Table.Row key={render.render_id}>
                    <Table.Cell>
                      <button
                        onClick={handleCopyId}
                        className="text-body text-neutral-500 font-mono"
                        title={`Click to copy full ID: ${render.render_id}`}
                      >
                        {truncatedId}
                      </button>
                    </Table.Cell>
                    <Table.Cell>
                      <span className="text-body-bold font-body-bold text-neutral-700">
                        {render.template_ref}
                      </span>
                    </Table.Cell>
                    <Table.Cell>{getStatusBadge(render.success)}</Table.Cell>
                    <Table.Cell>
                      <span className="text-body font-body text-neutral-500">
                        {formatLatency(render.duration_ms)}
                      </span>
                    </Table.Cell>
                    <Table.Cell>
                      <span className="text-body font-body text-neutral-500">
                        {formatRelativeTime(render.timestamp)}
                      </span>
                    </Table.Cell>
                    <Table.Cell>
                      <div className="flex grow shrink-0 basis-0 items-center justify-end">
                        <IconButton
                          icon={<FeatherDownload />}
                          onClick={handleDownload}
                          disabled={!render.success || !render.pdf_hash}
                        />
                      </div>
                    </Table.Cell>
                  </Table.Row>
                );
              })
            )}
          </Table>
        </div>
        <div className="flex w-full flex-col items-start gap-6 pb-10">
          <div className="flex w-full items-center justify-between">
            <span className="text-heading-3 font-heading-3 text-default-font">
              Templates
            </span>
            <Button
              icon={<FeatherPlus />}
              onClick={() => setNewTemplateModalOpen(true)}
            >
              New Template
            </Button>
          </div>
          <Tabs>
            <Tabs.Item active={true}>Most Used</Tabs.Item>
            <Tabs.Item>Recently Updated</Tabs.Item>
            <Tabs.Item>All Templates</Tabs.Item>
          </Tabs>
          <div className="w-full items-start gap-4 grid grid-cols-3">
            {templates.length === 0 ? (
              <div className="col-span-3 flex w-full items-center justify-center py-8">
                <span className="text-body font-body text-subtext-color">
                  No templates found
                </span>
              </div>
            ) : (
              templates.slice(0, 3).map((template) => (
                <div
                  key={template.name}
                  className="flex flex-col items-start gap-4 rounded-md border border-solid border-neutral-border bg-default-background px-4 py-4"
                >
                  <div className="flex w-full items-start justify-between">
                    <span className="text-body-bold font-body-bold text-default-font">
                      {template.name}
                    </span>
                    <Badge variant="success">Active</Badge>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        <NewTemplateModal
          open={newTemplateModalOpen}
          onOpenChange={setNewTemplateModalOpen}
        />
      </div>
    </DefaultPageLayout>
  );
}

export default RenderInsightsHub;
