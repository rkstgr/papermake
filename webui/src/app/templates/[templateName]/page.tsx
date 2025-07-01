"use client";

import React, { useState, useEffect, useCallback, useRef } from "react";
import { useParams, useSearchParams } from "next/navigation";
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
import { FeatherSend } from "@subframe/core";
import { FeatherRefreshCw } from "@subframe/core";
import { PDFViewer } from "@/ui/components/PDFViewer";
import { 
  templateApi, 
  renderApi, 
  formatRelativeTime,
  ApiError 
} from "@/lib/api";
import { 
  TemplateMetadataResponse, 
  PublishTemplateRequest,
  RenderRecord
} from "@/lib/types";

function TemplateStudio() {
  const params = useParams();
  const searchParams = useSearchParams();
  const templateName = params.templateName as string;
  const requestedVersion = searchParams.get('version');
  const isNewTemplate = searchParams.get('new') === 'true';
  
  // State management
  const [template, setTemplate] = useState<TemplateMetadataResponse | null>(null);
  const [templateContent, setTemplateContent] = useState("");
  const [previewBlob, setPreviewBlob] = useState<Blob | null>(null);
  const [schema, setSchema] = useState<any>(null);
  const [recentRenders, setRecentRenders] = useState<RenderRecord[]>([]);
  
  // UI state
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [publishing, setPublishing] = useState(false);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
  
  // Auto-save timer ref
  const autoSaveTimer = useRef<NodeJS.Timeout | null>(null);
  
  // Load template data
  useEffect(() => {
    const loadTemplate = async () => {
      try {
        setLoading(true);
        setError(null);
        
        if (isNewTemplate) {
          // Create a new template locally
          const newTemplate: TemplateMetadataResponse = {
            name: templateName.replace(/[-_]/g, ' ').replace(/\b\w/g, l => l.toUpperCase()),
            namespace: null,
            tag: 'draft',
            tags: ['draft'],
            manifest_hash: '',
            metadata: {
              name: templateName.replace(/[-_]/g, ' ').replace(/\b\w/g, l => l.toUpperCase()),
              author: 'User'
            },
            reference: `${templateName}:draft`
          };
          setTemplate(newTemplate);
          
          // Set initial template content
          setTemplateContent(`// ${templateName} template
#set page(paper: "a4", margin: 1in)
#set text(font: "Arial", size: 12pt)

= Welcome to ${templateName.replace(/[-_]/g, ' ')}

This is your new template. Start editing to customize it for your needs.

You can use variables in your content like this:
- Name: #name
- Date: #date

Happy templating!`);
          setHasUnsavedChanges(true); // Mark as unsaved since it's new
        } else {
          // Build reference string
          const reference = requestedVersion ? `${templateName}:${requestedVersion}` : templateName;
          
          // Load template metadata
          const templateData = await templateApi.get(reference);
          setTemplate(templateData);
          
          // For now, we'll use placeholder content since we don't have a content endpoint
          // In a real implementation, this would come from the template metadata or a separate endpoint
          setTemplateContent(`// ${templateName} template\n#set page(paper: "a4", margin: 1in)\n#set text(font: "Arial", size: 12pt)\n\n= ${templateName}\n\nThis is your template content.`);
        }
        
      } catch (err) {
        console.error("Failed to load template:", err);
        if (err instanceof ApiError && err.status === 404) {
          setError("Template not found");
        } else {
          setError("Failed to load template");
        }
      } finally {
        setLoading(false);
      }
    };

    if (templateName) {
      loadTemplate();
    }
  }, [templateName, requestedVersion, isNewTemplate]);

  // Load recent renders for this template
  useEffect(() => {
    const loadRecentRenders = async () => {
      try {
        const rendersResponse = await renderApi.list({ limit: 10 });
        // Filter renders for this template
        const templateRenders = rendersResponse.data.filter(render => 
          render.template_name === templateName || render.template_ref === templateName
        );
        setRecentRenders(templateRenders);
      } catch (err) {
        console.warn("Failed to load recent renders:", err);
        setRecentRenders([]);
      }
    };

    if (templateName && !isNewTemplate) {
      loadRecentRenders();
    }
  }, [templateName, isNewTemplate]);

  // Publish template functionality
  const publishTemplate = useCallback(async () => {
    if (!template || !hasUnsavedChanges) return;
    
    try {
      setPublishing(true);
      
      const publishRequest: PublishTemplateRequest = {
        main_typ: templateContent,
        metadata: {
          name: template.name,
          author: template.metadata.author
        },
        schema: schema
      };
      
      await templateApi.publishSimple(templateName, publishRequest, 'latest');
      setHasUnsavedChanges(false);
      
    } catch (err) {
      console.error("Failed to publish template:", err);
    } finally {
      setPublishing(false);
    }
  }, [template, templateName, templateContent, hasUnsavedChanges, schema]);

  // Content change handler
  const handleContentChange = useCallback((newContent: string) => {
    setTemplateContent(newContent);
    setHasUnsavedChanges(true);
  }, []);

  // Simple preview using render API
  const handleRender = useCallback(async () => {
    if (!template) return;
    
    try {
      const sampleData = generateSampleData();
      const result = await renderApi.render(templateName, { data: sampleData });
      
      // Download the rendered PDF
      const blob = await renderApi.downloadPdf(result.render_id);
      setPreviewBlob(blob);
      
    } catch (err) {
      console.error("Render failed:", err);
    }
  }, [template, templateName, generateSampleData]);

  // Generate sample data based on template schema or use defaults
  const generateSampleData = useCallback(() => {
    // Comprehensive sample data that covers common template fields
    const defaultSampleData = {
      // Personal/Entity information
      name: "John Doe",
      first_name: "John",
      last_name: "Doe",
      title: "Software Engineer",
      company: "Acme Corporation",
      organization: "Acme Corporation",
      
      // Contact information
      email: "john.doe@example.com",
      phone: "+1 (555) 123-4567",
      address: "123 Main St, Anytown, ST 12345",
      
      // Document information
      content: "Sample document content",
      description: "This is a sample document",
      subject: "Sample Subject",
      
      // Dates and numbers
      date: new Date().toISOString().split('T')[0],
      created_at: new Date().toISOString(),
      number: "001",
      id: "sample-001",
      
      // Invoice/Financial data
      invoice_number: "INV-001",
      amount: 1000,
      total: 1000,
      subtotal: 850,
      tax: 150,
      currency: "USD",
      
      // Arrays for testing lists
      items: [
        { name: "Item 1", quantity: 2, price: 100 },
        { name: "Item 2", quantity: 1, price: 200 }
      ],
      
      // Boolean flags
      paid: false,
      active: true,
      published: true
    };

    // If template has a schema, we could potentially extract required fields
    // For now, use the comprehensive default data
    return defaultSampleData;
  }, []);

  // Preview handler (simplified)
  const handlePreview = useCallback(async () => {
    if (!template) return;
    
    try {
      setPreviewing(true);
      await handleRender();
    } catch (err) {
      console.error("Preview failed:", err);
    } finally {
      setPreviewing(false);
    }
  }, [template, handleRender]);

  // Auto-preview disabled for now since it requires publishing to render

  // Loading state
  if (loading) {
    return (
      <DefaultPageLayout>
        <div className="max-w-none flex h-full w-full flex-col items-center justify-center">
          <div className="text-lg">Loading template...</div>
        </div>
      </DefaultPageLayout>
    );
  }

  // Error state
  if (error || !template) {
    return (
      <DefaultPageLayout>
        <div className="max-w-none flex h-full w-full flex-col items-center justify-center">
          <div className="text-lg text-red-600">{error || "Template not found"}</div>
        </div>
      </DefaultPageLayout>
    );
  }

  return (
    <DefaultPageLayout>
      <div className="max-w-none flex h-full w-full flex-col items-start">
        <div className="flex w-full grow shrink-0 basis-0 flex-col items-start">
          <div className="flex w-full items-center gap-4 border-b border-solid border-neutral-border px-8 py-4">
            <div className="flex grow shrink-0 basis-0 flex-col items-start gap-1">
              <div className="flex items-center gap-2">
                <span className="text-heading-2 font-heading-2 text-default-font">
                  {template.name}
                </span>
                {isNewTemplate ? (
                  <Badge variant="neutral">New</Badge>
                ) : (
                  <Badge variant="success">{template.tag}</Badge>
                )}
                {hasUnsavedChanges && <Badge variant="warning">Unsaved</Badge>}
                {publishing && <Badge variant="brand">Publishing...</Badge>}
              </div>
              <span className="text-body font-body text-subtext-color">
                Template by {template.metadata.author}
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
              onClick={() => {
                // Reset to initial content - would need to fetch from server in real implementation
                setHasUnsavedChanges(false);
              }}
              disabled={!hasUnsavedChanges}
            >
              Discard changes
            </Button>
            <Button
              variant="brand"
              icon={<FeatherSend />}
              onClick={publishTemplate}
              disabled={publishing || !hasUnsavedChanges}
            >
              {publishing ? "Publishing..." : "Publish"}
            </Button>
            <IconButton
              icon={<FeatherSettings2 />}
              onClick={(event: React.MouseEvent<HTMLButtonElement>) => {}}
            />
          </div>
          <div className="flex w-full grow shrink-0 basis-0 items-start gap-4 overflow-hidden px-6 py-4">
            <div className="flex grow shrink-0 basis-0 flex-col items-start gap-4 self-stretch">
              <div className="flex h-8 w-full flex-none items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="text-body-bold font-body-bold text-default-font">
                    Template Code
                  </span>
                </div>
                <Button
                  variant="neutral-tertiary"
                  size="small"
                  icon={<FeatherCode />}
                  onClick={handlePreview}
                  disabled={previewing}
                >
                  {previewing ? "Rendering..." : "Test Render"}
                </Button>
              </div>
              <TextArea
                className="w-full grow shrink-0 basis-0"
                variant="filled"
                label=""
                helpText=""
              >
                <TextArea.Input
                  className="min-h-[96px] w-full grow shrink-0 basis-0 font-mono"
                  placeholder="Enter your template code here..."
                  value={templateContent}
                  onChange={(event: React.ChangeEvent<HTMLTextAreaElement>) => {
                    handleContentChange(event.target.value);
                  }}
                />
              </TextArea>
            </div>
            <div className="flex w-px flex-none flex-col items-center gap-2 self-stretch bg-neutral-border" />
            <div className="flex grow shrink-0 basis-0 flex-col items-start gap-4 self-stretch">
              <div className="flex h-8 w-full flex-none items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="text-body-bold font-body-bold text-default-font">
                    Preview
                  </span>
                  {previewing && <Badge variant="neutral">Generating...</Badge>}
                </div>
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
                              Recent Renders ({recentRenders.length})
                            </span>
                          </div>
                          {recentRenders.slice(0, 5).map((render) => (
                            <DropdownMenu.DropdownItem 
                              key={render.render_id} 
                              icon={<FeatherFile />}
                            >
                              Render {render.render_id.substring(0, 8)}
                            </DropdownMenu.DropdownItem>
                          ))}
                        </DropdownMenu>
                      </SubframeCore.DropdownMenu.Content>
                    </SubframeCore.DropdownMenu.Portal>
                  </SubframeCore.DropdownMenu.Root>
                  <IconButton
                    size="small"
                    icon={<FeatherRefreshCw />}
                    onClick={handlePreview}
                    disabled={previewing}
                    title="Refresh preview"
                  />
                  <IconButton
                    size="small"
                    icon={<FeatherDownload />}
                    onClick={() => {
                      if (previewBlob) {
                        const url = URL.createObjectURL(previewBlob);
                        const a = document.createElement('a');
                        a.href = url;
                        a.download = `${template.name}-preview.pdf`;
                        document.body.appendChild(a);
                        a.click();
                        document.body.removeChild(a);
                        URL.revokeObjectURL(url);
                      }
                    }}
                    disabled={!previewBlob}
                  />
                  <IconButton
                    size="small"
                    icon={<FeatherMaximize2 />}
                    onClick={() => {
                      if (previewBlob) {
                        const url = URL.createObjectURL(previewBlob);
                        window.open(url, '_blank');
                        // URL will be cleaned up when the new window closes
                      }
                    }}
                    disabled={!previewBlob}
                  />
                </div>
              </div>
              <PDFViewer 
                blob={previewBlob} 
                className="w-full grow shrink-0 basis-0 rounded-md border border-dashed border-neutral-border bg-neutral-50"
              />
            </div>
          </div>
        </div>
        <div className="flex w-full flex-col items-start gap-2 px-1 py-1">
          <div className="flex w-full items-center justify-between border-t border-solid border-neutral-border bg-default-background px-6 py-3">
            <span className="text-caption font-caption text-subtext-color">
              {hasUnsavedChanges ? "Unsaved changes" : `Template: ${template.reference}`}
            </span>
            <Button
              icon={<FeatherUpload />}
              onClick={publishTemplate}
              disabled={publishing || !hasUnsavedChanges}
            >
              {publishing ? "Publishing..." : "Publish"}
            </Button>
          </div>
        </div>
      </div>
    </DefaultPageLayout>
  );
}

export default TemplateStudio;
