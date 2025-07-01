"use client";

import React, { useState } from "react";
import { useRouter } from "next/navigation";
import { DialogLayout } from "@/ui/layouts/DialogLayout";
import { TextField } from "@/ui/components/TextField";
import { Button } from "@/ui/components/Button";

interface NewTemplateModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

function NewTemplateModal({ open, onOpenChange }: NewTemplateModalProps) {
  const router = useRouter();
  const [templateName, setTemplateName] = useState("");
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleCreate = () => {
    if (!templateName.trim()) {
      setError("Template name is required");
      return;
    }

    // Validate template name format (no spaces, alphanumeric with hyphens/underscores)
    const validNamePattern = /^[a-zA-Z0-9_-]+$/;
    if (!validNamePattern.test(templateName)) {
      setError("Template name can only contain letters, numbers, hyphens, and underscores");
      return;
    }

    // Close modal and navigate to the new template editor
    onOpenChange(false);
    setTemplateName("");
    router.push(`/templates/${templateName}?new=true`);
  };

  const handleCancel = () => {
    setTemplateName("");
    setError(null);
    onOpenChange(false);
  };

  const handleNameChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;
    setTemplateName(value);
    
    // Clear error when user starts typing
    if (error) {
      setError(null);
    }
  };

  return (
    <DialogLayout open={open} onOpenChange={onOpenChange}>
      <div className="flex h-full w-full flex-col items-start gap-6 bg-default-background px-6 py-6">
        <div className="flex w-full flex-col items-start gap-2">
          <span className="text-heading-3 font-heading-3 text-default-font">
            Create new template
          </span>
          <span className="text-body font-body text-subtext-color">
            Enter a machine-readable name for your template (letters, numbers, hyphens, and underscores only)
          </span>
        </div>
        <TextField
          className="h-auto w-full flex-none"
          label="Template name"
          helpText={error || "Use lowercase letters, numbers, hyphens, and underscores (e.g., invoice-template, my_report)"}
          error={!!error}
        >
          <TextField.Input
            placeholder="e.g., invoice-template"
            value={templateName}
            onChange={handleNameChange}
          />
        </TextField>
        <div className="flex w-full items-center justify-end gap-2">
          <Button
            variant="neutral-secondary"
            onClick={handleCancel}
          >
            Cancel
          </Button>
          <Button 
            onClick={handleCreate}
            disabled={!templateName.trim()}
          >
            Create template
          </Button>
        </div>
      </div>
    </DialogLayout>
  );
}

export default NewTemplateModal;