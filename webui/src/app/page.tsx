"use client";

import React from "react";
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

function RenderInsightsHub() {
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
                24,521
              </span>
              <Badge variant="success" icon={<FeatherArrowUp />}>
                15% vs last week
              </Badge>
            </div>
          </div>
          <div className="flex grow shrink-0 basis-0 flex-col items-start gap-4 rounded-md border border-solid border-neutral-border bg-default-background px-4 py-4">
            <span className="line-clamp-1 w-full text-caption-bold font-caption-bold text-subtext-color">
              Average Render Time
            </span>
            <div className="flex w-full flex-col items-start gap-2">
              <span className="text-heading-2 font-heading-2 text-default-font">
                1.2s
              </span>
              <Badge variant="success" icon={<FeatherArrowDown />}>
                0.3s improvement
              </Badge>
            </div>
          </div>
          <div className="flex grow shrink-0 basis-0 flex-col items-start gap-4 rounded-md border border-solid border-neutral-border bg-default-background px-4 py-4">
            <span className="line-clamp-1 w-full text-caption-bold font-caption-bold text-subtext-color">
              Active Templates
            </span>
            <div className="flex w-full flex-col items-start gap-2">
              <span className="text-heading-2 font-heading-2 text-default-font">
                142
              </span>
              <Badge variant="neutral" icon={<FeatherPlus />}>
                4 new this week
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
                <Table.HeaderCell>Template</Table.HeaderCell>
                <Table.HeaderCell>Status</Table.HeaderCell>
                <Table.HeaderCell>Time</Table.HeaderCell>
                <Table.HeaderCell>Size</Table.HeaderCell>
                <Table.HeaderCell>User</Table.HeaderCell>
                <Table.HeaderCell />
              </Table.HeaderRow>
            }
          >
            <Table.Row>
              <Table.Cell>
                <span className="text-body-bold font-body-bold text-neutral-700">
                  Invoice Template
                </span>
              </Table.Cell>
              <Table.Cell>
                <Badge variant="success" icon={<FeatherCheckCircle />}>
                  Complete
                </Badge>
              </Table.Cell>
              <Table.Cell>
                <span className="text-body font-body text-neutral-500">
                  1.1s
                </span>
              </Table.Cell>
              <Table.Cell>
                <span className="text-body font-body text-neutral-500">
                  245 KB
                </span>
              </Table.Cell>
              <Table.Cell>
                <div className="flex items-center gap-2">
                  <Avatar size="small" image="">
                    JD
                  </Avatar>
                  <span className="text-body font-body text-neutral-500">
                    John Doe
                  </span>
                </div>
              </Table.Cell>
              <Table.Cell>
                <div className="flex grow shrink-0 basis-0 items-center justify-end">
                  <IconButton
                    icon={<FeatherDownload />}
                    onClick={(event: React.MouseEvent<HTMLButtonElement>) => {}}
                  />
                </div>
              </Table.Cell>
            </Table.Row>
            <Table.Row>
              <Table.Cell>
                <span className="text-body-bold font-body-bold text-neutral-700">
                  Report Template
                </span>
              </Table.Cell>
              <Table.Cell>
                <Badge variant="success" icon={<FeatherCheckCircle />}>
                  Complete
                </Badge>
              </Table.Cell>
              <Table.Cell>
                <span className="text-body font-body text-neutral-500">
                  0.8s
                </span>
              </Table.Cell>
              <Table.Cell>
                <span className="text-body font-body text-neutral-500">
                  180 KB
                </span>
              </Table.Cell>
              <Table.Cell>
                <div className="flex items-center gap-2">
                  <Avatar size="small" image="">
                    AS
                  </Avatar>
                  <span className="text-body font-body text-neutral-500">
                    Alice Smith
                  </span>
                </div>
              </Table.Cell>
              <Table.Cell>
                <div className="flex grow shrink-0 basis-0 items-center justify-end">
                  <IconButton
                    icon={<FeatherDownload />}
                    onClick={(event: React.MouseEvent<HTMLButtonElement>) => {}}
                  />
                </div>
              </Table.Cell>
            </Table.Row>
            <Table.Row>
              <Table.Cell>
                <span className="text-body-bold font-body-bold text-neutral-700">
                  Contract Template
                </span>
              </Table.Cell>
              <Table.Cell>
                <Badge variant="warning" icon={<FeatherClock />}>
                  Processing
                </Badge>
              </Table.Cell>
              <Table.Cell>
                <span className="text-body font-body text-neutral-500">
                  2.1s
                </span>
              </Table.Cell>
              <Table.Cell>
                <span className="text-body font-body text-neutral-500">
                  320 KB
                </span>
              </Table.Cell>
              <Table.Cell>
                <div className="flex items-center gap-2">
                  <Avatar size="small" image="">
                    RJ
                  </Avatar>
                  <span className="text-body font-body text-neutral-500">
                    Robert Johnson
                  </span>
                </div>
              </Table.Cell>
              <Table.Cell>
                <div className="flex grow shrink-0 basis-0 items-center justify-end">
                  <IconButton
                    icon={<FeatherDownload />}
                    onClick={(event: React.MouseEvent<HTMLButtonElement>) => {}}
                  />
                </div>
              </Table.Cell>
            </Table.Row>
          </Table>
        </div>
        <div className="flex w-full flex-col items-start gap-6">
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
            <div className="flex flex-col items-start gap-4 rounded-md border border-solid border-neutral-border bg-default-background px-4 py-4">
              <div className="flex w-full items-start justify-between">
                <span className="text-body-bold font-body-bold text-default-font">
                  Invoice Template
                </span>
                <Badge variant="success">Active</Badge>
              </div>
              <div className="flex items-center gap-2">
                <FeatherBarChart2 className="text-body font-body text-subtext-color" />
                <span className="text-body font-body text-subtext-color">
                  2,451 uses
                </span>
              </div>
              <span className="text-caption font-caption text-subtext-color">
                Last updated: 2 days ago
              </span>
            </div>
            <div className="flex flex-col items-start gap-4 rounded-md border border-solid border-neutral-border bg-default-background px-4 py-4">
              <div className="flex w-full items-start justify-between">
                <span className="text-body-bold font-body-bold text-default-font">
                  Report Template
                </span>
                <Badge variant="success">Active</Badge>
              </div>
              <div className="flex items-center gap-2">
                <FeatherBarChart2 className="text-body font-body text-subtext-color" />
                <span className="text-body font-body text-subtext-color">
                  1,832 uses
                </span>
              </div>
              <span className="text-caption font-caption text-subtext-color">
                Last updated: 5 days ago
              </span>
            </div>
            <div className="flex flex-col items-start gap-4 rounded-md border border-solid border-neutral-border bg-default-background px-4 py-4">
              <div className="flex w-full items-start justify-between">
                <span className="text-body-bold font-body-bold text-default-font">
                  Contract Template
                </span>
                <Badge>Draft</Badge>
              </div>
              <div className="flex items-center gap-2">
                <FeatherBarChart2 className="text-body font-body text-subtext-color" />
                <span className="text-body font-body text-subtext-color">
                  845 uses
                </span>
              </div>
              <span className="text-caption font-caption text-subtext-color">
                Last updated: 1 week ago
              </span>
            </div>
          </div>
        </div>
      </div>
    </DefaultPageLayout>
  );
}

export default RenderInsightsHub;
