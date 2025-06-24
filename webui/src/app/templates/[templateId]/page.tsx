"use client";

import React from "react";
import { DefaultPageLayout } from "@/ui/layouts/DefaultPageLayout";
import { Badge } from "@/ui/components/Badge";
import { Button } from "@/ui/components/Button";
import { FeatherHistory } from "@subframe/core";
import { FeatherRotateCcw } from "@subframe/core";
import { IconButton } from "@/ui/components/IconButton";
import { FeatherSettings2 } from "@subframe/core";
import { FeatherCode } from "@subframe/core";
import { TextArea } from "@/ui/components/TextArea";
import { TextField } from "@/ui/components/TextField";
import { FeatherSearch } from "@subframe/core";
import { DropdownMenu } from "@/ui/components/DropdownMenu";
import { FeatherPlus } from "@subframe/core";
import { FeatherFile } from "@subframe/core";
import * as SubframeCore from "@subframe/core";
import { FeatherStamp } from "@subframe/core";
import { FeatherChevronDown } from "@subframe/core";
import { FeatherDownload } from "@subframe/core";
import { FeatherMaximize2 } from "@subframe/core";
import { FeatherUpload } from "@subframe/core";

function TemplateStudio() {
  return (
    <DefaultPageLayout>
      <div className="max-w-none flex h-full w-full flex-col items-start">
        <div className="flex w-full grow shrink-0 basis-0 flex-col items-start">
          <div className="flex w-full items-center gap-4 border-b border-solid border-neutral-border px-8 py-4">
            <div className="flex grow shrink-0 basis-0 flex-col items-start gap-1">
              <div className="flex items-center gap-2">
                <span className="text-heading-2 font-heading-2 text-default-font">
                  Invoice Template
                </span>
                <Badge variant="success">Published</Badge>
                <Badge variant="neutral">Draft</Badge>
              </div>
              <span className="text-body font-body text-subtext-color">
                Last edited 12 minutes ago
              </span>
            </div>
            <Button
              variant="neutral-secondary"
              icon={<FeatherHistory />}
              onClick={(event: React.MouseEvent<HTMLButtonElement>) => {}}
            >
              View History
            </Button>
            <Button
              variant="neutral-secondary"
              icon={<FeatherRotateCcw />}
              onClick={(event: React.MouseEvent<HTMLButtonElement>) => {}}
            >
              Discard changes
            </Button>
            <IconButton
              icon={<FeatherSettings2 />}
              onClick={(event: React.MouseEvent<HTMLButtonElement>) => {}}
            />
          </div>
          <div className="flex w-full grow shrink-0 basis-0 items-start gap-4 overflow-hidden px-6 py-4">
            <div className="flex grow shrink-0 basis-0 flex-col items-start gap-4 self-stretch">
              <div className="flex h-8 w-full flex-none items-center justify-between">
                <span className="text-body-bold font-body-bold text-default-font">
                  Template Code
                </span>
                <Button
                  variant="neutral-tertiary"
                  size="small"
                  icon={<FeatherCode />}
                  onClick={(event: React.MouseEvent<HTMLButtonElement>) => {}}
                >
                  Format
                </Button>
              </div>
              <TextArea
                className="w-full grow shrink-0 basis-0"
                variant="filled"
                label=""
                helpText=""
              >
                <TextArea.Input
                  className="min-h-[96px] w-full grow shrink-0 basis-0"
                  placeholder="Enter your template code here..."
                  value=""
                  onChange={(
                    event: React.ChangeEvent<HTMLTextAreaElement>,
                  ) => {}}
                />
              </TextArea>
            </div>
            <div className="flex w-px flex-none flex-col items-center gap-2 self-stretch bg-neutral-border" />
            <div className="flex grow shrink-0 basis-0 flex-col items-start gap-4 self-stretch">
              <div className="flex h-8 w-full flex-none items-center justify-between">
                <span className="text-body-bold font-body-bold text-default-font">
                  Preview
                </span>
                <div className="flex grow shrink-0 basis-0 items-center justify-end gap-2 px-1 py-1">
                  <SubframeCore.DropdownMenu.Root>
                    <SubframeCore.DropdownMenu.Trigger asChild={true}>
                      <Button
                        variant="neutral-tertiary"
                        size="small"
                        icon={<FeatherStamp />}
                        iconRight={<FeatherChevronDown />}
                        onClick={(
                          event: React.MouseEvent<HTMLButtonElement>,
                        ) => {}}
                      >
                        Preview data
                      </Button>
                    </SubframeCore.DropdownMenu.Trigger>
                    <SubframeCore.DropdownMenu.Portal>
                      <SubframeCore.DropdownMenu.Content
                        side="bottom"
                        align="start"
                        sideOffset={4}
                        asChild={true}
                      >
                        <DropdownMenu>
                          <div className="flex w-full flex-col items-start gap-2 px-3 py-2">
                            <TextField
                              className="h-auto w-full flex-none"
                              variant="filled"
                              label=""
                              helpText=""
                              icon={<FeatherSearch />}
                            >
                              <TextField.Input
                                placeholder="Search renders..."
                                value=""
                                onChange={(
                                  event: React.ChangeEvent<HTMLInputElement>,
                                ) => {}}
                              />
                            </TextField>
                          </div>
                          <DropdownMenu.DropdownItem icon={<FeatherPlus />}>
                            Create custom data
                          </DropdownMenu.DropdownItem>
                          <DropdownMenu.DropdownDivider />
                          <div className="flex w-full flex-col items-start gap-1 px-2 py-1">
                            <span className="text-caption font-caption text-subtext-color">
                              Recent Renders
                            </span>
                          </div>
                          <DropdownMenu.DropdownItem icon={<FeatherFile />}>
                            Invoice #1234
                          </DropdownMenu.DropdownItem>
                          <DropdownMenu.DropdownItem icon={<FeatherFile />}>
                            Invoice #1235
                          </DropdownMenu.DropdownItem>
                          <DropdownMenu.DropdownItem icon={<FeatherFile />}>
                            Invoice #1236
                          </DropdownMenu.DropdownItem>
                        </DropdownMenu>
                      </SubframeCore.DropdownMenu.Content>
                    </SubframeCore.DropdownMenu.Portal>
                  </SubframeCore.DropdownMenu.Root>
                  <IconButton
                    size="small"
                    icon={<FeatherDownload />}
                    onClick={(event: React.MouseEvent<HTMLButtonElement>) => {}}
                  />
                  <IconButton
                    size="small"
                    icon={<FeatherMaximize2 />}
                    onClick={(event: React.MouseEvent<HTMLButtonElement>) => {}}
                  />
                </div>
              </div>
              <div className="flex w-full grow shrink-0 basis-0 items-center justify-center rounded-md border border-dashed border-neutral-border bg-neutral-50" />
            </div>
          </div>
        </div>
        <div className="flex w-full flex-col items-start gap-2 px-1 py-1">
          <div className="flex w-full items-center justify-between border-t border-solid border-neutral-border bg-default-background px-6 py-3">
            <span className="text-caption font-caption text-subtext-color">
              Last saved 8 seconds ago
            </span>
            <Button
              icon={<FeatherUpload />}
              onClick={(event: React.MouseEvent<HTMLButtonElement>) => {}}
            >
              Publish
            </Button>
          </div>
        </div>
      </div>
    </DefaultPageLayout>
  );
}

export default TemplateStudio;
