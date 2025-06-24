"use client";

import React, { useEffect, useState } from "react";
import { DefaultPageLayout } from "@/ui/layouts/DefaultPageLayout";
import { Badge } from "@/ui/components/Badge";
import { FeatherArrowUp } from "@subframe/core";
import { FeatherArrowDown } from "@subframe/core";
import { FeatherPlus } from "@subframe/core";
import { ToggleGroup } from "@/ui/components/ToggleGroup";
import { AreaChart } from "@/ui/components/AreaChart";
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
  analyticsApi,
  renderApi,
  templateApi,
  formatLatency,
  formatRelativeTime,
  formatFileSize,
} from "@/lib/api";
import {
  DashboardMetrics,
  RenderJobSummary,
  TemplateSummary,
} from "@/lib/types";

function RenderInsightsHub() {
  const [dashboardData, setDashboardData] = useState<DashboardMetrics | null>(
    null,
  );
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchDashboardData = async () => {
      try {
        setLoading(true);
        const data = await analyticsApi.getDashboard();
        setDashboardData(data);
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

  if (error || !dashboardData) {
    return (
      <DefaultPageLayout>
        <div className="container max-w-none flex h-full w-full flex-col items-center justify-center">
          <div className="text-lg text-red-600">
            {error || "Failed to load data"}
          </div>
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
                {dashboardData.total_renders_24h.toLocaleString()}
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
                {dashboardData.p90_latency_ms
                  ? formatLatency(dashboardData.p90_latency_ms)
                  : "N/A"}
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
                {dashboardData.popular_templates.length}
              </span>
              <Badge variant="neutral" icon={<FeatherPlus />}>
                {dashboardData.new_templates.length} new
              </Badge>
            </div>
          </div>
        </div>
        <div className="flex w-full flex-col items-start gap-6 rounded-md border border-solid border-neutral-border bg-default-background px-6 py-6 shadow-sm">
          <div className="flex w-full items-center gap-2">
            <span className="grow shrink-0 basis-0 text-heading-3 font-heading-3 text-default-font">
              Render Volume
            </span>
            <ToggleGroup value="" onValueChange={(value: string) => {}}>
              <ToggleGroup.Item icon={null} value="019b3d5e">
                24h
              </ToggleGroup.Item>
              <ToggleGroup.Item icon={null} value="da58ad72">
                7d
              </ToggleGroup.Item>
              <ToggleGroup.Item icon={null} value="3a340ab5">
                30d
              </ToggleGroup.Item>
            </ToggleGroup>
          </div>
          <AreaChart
            className="h-64 w-full flex-none"
            categories={["Biology", "Business", "Psychology"]}
            data={[
              { Year: "2015", Psychology: 120, Business: 110, Biology: 100 },
              { Year: "2016", Psychology: 130, Business: 95, Biology: 105 },
              { Year: "2017", Psychology: 115, Business: 105, Biology: 110 },
              { Year: "2018", Psychology: 125, Business: 120, Biology: 90 },
              { Year: "2019", Psychology: 110, Business: 130, Biology: 85 },
              { Year: "2020", Psychology: 135, Business: 100, Biology: 95 },
              { Year: "2021", Psychology: 105, Business: 115, Biology: 120 },
              { Year: "2022", Psychology: 140, Business: 125, Biology: 130 },
            ]}
            index={"Year"}
          />
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
            {dashboardData.recent_renders.map((render) => {
              const getStatusBadge = (status: string) => {
                switch (status) {
                  case "completed":
                    return (
                      <Badge variant="success" icon={<FeatherCheckCircle />}>
                        Complete
                      </Badge>
                    );
                  case "processing":
                    return (
                      <Badge variant="warning" icon={<FeatherClock />}>
                        Processing
                      </Badge>
                    );
                  case "queued":
                    return (
                      <Badge variant="neutral" icon={<FeatherClock />}>
                        Queued
                      </Badge>
                    );
                  case "failed":
                    return <Badge variant="error">Failed</Badge>;
                  default:
                    return <Badge variant="neutral">{status}</Badge>;
                }
              };

              const handleDownload = async () => {
                if (render.status === "completed" && render.pdf_url) {
                  try {
                    const blob = await renderApi.downloadPdf(render.id);
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement("a");
                    a.href = url;
                    a.download = `${render.template_id}-${render.id}.pdf`;
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
                  await navigator.clipboard.writeText(render.id);
                } catch (error) {
                  console.error("Failed to copy ID:", error);
                }
              };

              // Truncate render ID to first 8 characters
              const truncatedId = render.id.substring(0, 8);

              return (
                <Table.Row key={render.id}>
                  <Table.Cell>
                    <button
                      onClick={handleCopyId}
                      className="text-body text-neutral-500 font-mono"
                      title={`Click to copy full ID: ${render.id}`}
                    >
                      {truncatedId}
                    </button>
                  </Table.Cell>
                  <Table.Cell>
                    <span className="text-body-bold font-body-bold text-neutral-700">
                      {render.template_id}
                    </span>
                  </Table.Cell>
                  <Table.Cell>{getStatusBadge(render.status)}</Table.Cell>
                  <Table.Cell>
                    <span className="text-body font-body text-neutral-500">
                      {formatLatency(render.rendering_latency)}
                    </span>
                  </Table.Cell>
                  <Table.Cell>
                    <span className="text-body font-body text-neutral-500">
                      {render.completed_at
                        ? formatRelativeTime(render.completed_at)
                        : formatRelativeTime(render.created_at)}
                    </span>
                  </Table.Cell>
                  <Table.Cell>
                    <div className="flex grow shrink-0 basis-0 items-center justify-end">
                      <IconButton
                        icon={<FeatherDownload />}
                        onClick={handleDownload}
                        disabled={
                          render.status !== "completed" || !render.pdf_url
                        }
                      />
                    </div>
                  </Table.Cell>
                </Table.Row>
              );
            })}
          </Table>
        </div>
        <div className="flex w-full flex-col items-start gap-6 pb-10">
          <div className="flex w-full items-center justify-between">
            <span className="text-heading-3 font-heading-3 text-default-font">
              Templates
            </span>
            <Button
              icon={<FeatherPlus />}
              onClick={(event: React.MouseEvent<HTMLButtonElement>) => {}}
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
            {dashboardData.popular_templates.slice(0, 3).map((template) => (
              <div
                key={template.template_id}
                className="flex flex-col items-start gap-4 rounded-md border border-solid border-neutral-border bg-default-background px-4 py-4"
              >
                <div className="flex w-full items-start justify-between">
                  <span className="text-body-bold font-body-bold text-default-font">
                    {template.template_name}
                  </span>
                  <Badge variant="success">Active</Badge>
                </div>
                <div className="flex items-center gap-2">
                  <FeatherBarChart2 className="text-body font-body text-subtext-color" />
                  <span className="text-body font-body text-subtext-color">
                    {template.uses_24h.toLocaleString()} uses (24h)
                  </span>
                </div>
                <span className="text-caption font-caption text-subtext-color">
                  Last updated: {formatRelativeTime(template.published_at)}
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </DefaultPageLayout>
  );
}

export default RenderInsightsHub;
