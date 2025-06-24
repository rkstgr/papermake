"use client";

import React, { useState, useEffect, useCallback, useRef } from "react";
import { useParams } from "next/navigation";
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
import { FeatherRefreshCw } from "@subframe/core";
import { 
  templateApi, 
  renderApi, 
  formatRelativeTime,
  ApiError 
} from "@/lib/api";
import { 
  TemplateDetails, 
  RenderJobSummary,
  TemplateValidationResponse 
} from "@/lib/types";

function TemplateStudio() {
  const params = useParams();
  const templateId = params.templateId as string;
  
  // State management
  const [template, setTemplate] = useState<TemplateDetails | null>(null);
  const [templateContent, setTemplateContent] = useState("");
  const [recentRenders, setRecentRenders] = useState<RenderJobSummary[]>([]);
  const [previewBlob, setPreviewBlob] = useState<string | null>(null);
  const [validationResult, setValidationResult] = useState<TemplateValidationResponse | null>(null);
  
  // UI state
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [previewing, setPreviewing] = useState(false);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
  
  // Auto-save timer ref
  const autoSaveTimer = useRef<NodeJS.Timeout | null>(null);
  
  // Load template data
  useEffect(() => {
    const loadTemplate = async () => {
      try {
        setLoading(true);
        setError(null);
        
        const templateData = await templateApi.get(templateId);
        setTemplate(templateData);
        setTemplateContent(templateData.content);
        
        // Load recent renders for this template (handle errors gracefully)
        try {
          const renders = await renderApi.list({
            template_id: templateId,
            limit: 10
          });
          setRecentRenders(renders.data);
        } catch (renderErr) {
          console.warn("Failed to load recent renders:", renderErr);
          // Don't fail the whole page load if renders can't be loaded
          setRecentRenders([]);
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

    if (templateId) {
      loadTemplate();
    }
  }, [templateId]);

  // Auto-save functionality
  const saveTemplate = useCallback(async () => {
    if (!template || !hasUnsavedChanges) return;
    
    try {
      setSaving(true);
      // Note: Update API would need to be implemented in the server
      // For now, we'll just validate the content
      await templateApi.validate({
        content: templateContent,
        schema: template.schema
      });
      setHasUnsavedChanges(false);
    } catch (err) {
      console.error("Failed to save template:", err);
    } finally {
      setSaving(false);
    }
  }, [template, templateContent, hasUnsavedChanges]);

  // Content change handler with debounced auto-save
  const handleContentChange = useCallback((newContent: string) => {
    setTemplateContent(newContent);
    setHasUnsavedChanges(true);
    
    // Clear existing timer
    if (autoSaveTimer.current) {
      clearTimeout(autoSaveTimer.current);
    }
    
    // Set new timer for auto-save
    autoSaveTimer.current = setTimeout(() => {
      saveTemplate();
    }, 2000);
  }, [saveTemplate]);

  // Validation handler
  const handleValidation = useCallback(async () => {
    if (!template) return;
    
    try {
      const result = await templateApi.validate({
        content: templateContent,
        schema: template.schema
      });
      setValidationResult(result);
    } catch (err) {
      console.error("Validation failed:", err);
    }
  }, [template, templateContent]);

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

  // Preview handler
  const handlePreview = useCallback(async () => {
    if (!template) return;
    
    try {
      setPreviewing(true);
      
      const sampleData = generateSampleData();
      
      const blob = await templateApi.preview({
        content: templateContent,
        data: sampleData,
        schema: template.schema
      });
      
      const url = URL.createObjectURL(blob);
      if (previewBlob) {
        URL.revokeObjectURL(previewBlob);
      }
      setPreviewBlob(url);
      
    } catch (err) {
      console.error("Preview failed:", err);
      // Don't show the error to user, just log it
      // Preview failing shouldn't break the editor experience
    } finally {
      setPreviewing(false);
    }
  }, [template, templateContent, previewBlob, generateSampleData]);

  // Auto-preview when content changes (debounced) - only if content is substantial
  useEffect(() => {
    const timer = setTimeout(() => {
      if (templateContent && template && templateContent.length > 50) {
        handlePreview();
      }
    }, 3000); // Increased delay to reduce server load
    
    return () => clearTimeout(timer);
  }, [templateContent, template, handlePreview]);

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
                <Badge variant="success">v{template.version}</Badge>
                {hasUnsavedChanges && <Badge variant="warning">Unsaved</Badge>}
                {saving && <Badge variant="neutral">Saving...</Badge>}
              </div>
              <span className="text-body font-body text-subtext-color">
                Last updated {formatRelativeTime(template.published_at)} by {template.author}
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
                if (template) {
                  setTemplateContent(template.content);
                  setHasUnsavedChanges(false);
                }
              }}
              disabled={!hasUnsavedChanges}
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
                <div className="flex items-center gap-2">
                  <span className="text-body-bold font-body-bold text-default-font">
                    Template Code
                  </span>
                  {validationResult && !validationResult.valid && (
                    <Badge variant="error">{validationResult.errors.length} errors</Badge>
                  )}
                  {validationResult && validationResult.warnings.length > 0 && (
                    <Badge variant="warning">{validationResult.warnings.length} warnings</Badge>
                  )}
                </div>
                <Button
                  variant="neutral-tertiary"
                  size="small"
                  icon={<FeatherCode />}
                  onClick={handleValidation}
                >
                  Validate
                </Button>
              </div>
              <TextArea
                className="w-full grow shrink-0 basis-0"
                variant="filled"
                label=""
                helpText={validationResult && !validationResult.valid ? validationResult.errors[0]?.message : ""}
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
                              key={render.id} 
                              icon={<FeatherFile />}
                            >
                              Render {render.id.substring(0, 8)}
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
                        const a = document.createElement('a');
                        a.href = previewBlob;
                        a.download = `${template.name}-preview.pdf`;
                        document.body.appendChild(a);
                        a.click();
                        document.body.removeChild(a);
                      }
                    }}
                    disabled={!previewBlob}
                  />
                  <IconButton
                    size="small"
                    icon={<FeatherMaximize2 />}
                    onClick={() => {
                      if (previewBlob) {
                        window.open(previewBlob, '_blank');
                      }
                    }}
                    disabled={!previewBlob}
                  />
                </div>
              </div>
              <div className="flex w-full grow shrink-0 basis-0 items-center justify-center rounded-md border border-dashed border-neutral-border bg-neutral-50">
                {previewBlob ? (
                  <iframe
                    src={previewBlob}
                    className="w-full h-full rounded-md"
                    title="PDF Preview"
                  />
                ) : (
                  <div className="flex flex-col items-center gap-2 text-subtext-color">
                    {previewing ? (
                      <>
                        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-subtext-color"></div>
                        <span>Generating preview...</span>
                      </>
                    ) : (
                      <>
                        <FeatherFile className="h-8 w-8" />
                        <span>Preview will appear here</span>
                      </>
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
        <div className="flex w-full flex-col items-start gap-2 px-1 py-1">
          <div className="flex w-full items-center justify-between border-t border-solid border-neutral-border bg-default-background px-6 py-3">
            <span className="text-caption font-caption text-subtext-color">
              {saving ? "Saving..." : hasUnsavedChanges ? "Unsaved changes" : `Last saved ${formatRelativeTime(template.published_at)}`}
            </span>
            <Button
              icon={<FeatherUpload />}
              onClick={saveTemplate}
              disabled={saving || !hasUnsavedChanges}
            >
              {saving ? "Saving..." : "Save"}
            </Button>
          </div>
        </div>
      </div>
    </DefaultPageLayout>
  );
}

export default TemplateStudio;
