"use client";

import { useState, useMemo, useEffect } from "react";
import { Document, Page, pdfjs } from "react-pdf";
import "react-pdf/dist/esm/Page/AnnotationLayer.css";

pdfjs.GlobalWorkerOptions.workerSrc = new URL(
  "pdfjs-dist/build/pdf.worker.min.mjs",
  import.meta.url
).toString();

interface PDFViewerProps {
  blob: Blob | null;
  className?: string;
}

export function PDFViewer({ blob, className = "" }: PDFViewerProps) {
  const [numPages, setNumPages] = useState<number>(0);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  // Convert blob to URL for react-pdf
  const pdfUrl = useMemo(() => {
    if (!blob) return null;
    return URL.createObjectURL(blob);
  }, [blob]);

  // Cleanup URL when component unmounts or blob changes
  useEffect(() => {
    return () => {
      if (pdfUrl) {
        URL.revokeObjectURL(pdfUrl);
      }
    };
  }, [pdfUrl]);

  function onDocumentLoadSuccess({ numPages }: { numPages: number }) {
    setNumPages(numPages);
    setLoading(false);
    setError(null);
  }

  function onDocumentLoadError(error: Error) {
    console.error("Failed to load PDF:", error);
    setError("Failed to load PDF");
    setLoading(false);
  }

  function onLoadStart() {
    setLoading(true);
    setError(null);
  }

  if (!blob) {
    return (
      <div className={`flex items-center justify-center ${className}`}>
        <div className="flex flex-col items-center gap-2 text-subtext-color">
          <svg
            className="h-8 w-8"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
          <span>Preview will appear here</span>
        </div>
      </div>
    );
  }

  if (loading) {
    return (
      <div className={`flex items-center justify-center ${className}`}>
        <div className="flex flex-col items-center gap-2 text-subtext-color">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-subtext-color"></div>
          <span>Loading PDF...</span>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className={`flex items-center justify-center ${className}`}>
        <div className="text-red-500">{error}</div>
      </div>
    );
  }

  return (
    <div className={`relative flex flex-col h-full ${className}`} style={{ minHeight: '400px' }}>
      {/* Page counter for multi-page documents */}
      {numPages > 1 && (
        <div className="absolute top-2 right-2 z-10 bg-white/90 backdrop-blur-sm rounded-md px-3 py-1 shadow-sm">
          <span className="text-sm text-gray-700">
            {numPages} pages
          </span>
        </div>
      )}

      {/* PDF Document - Scrollable container */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden p-4 bg-gray-50 rounded-md" style={{ maxHeight: 'calc(100% - 8px)' }}>
        <Document
          file={pdfUrl}
          onLoadStart={onLoadStart}
          onLoadSuccess={onDocumentLoadSuccess}
          onLoadError={onDocumentLoadError}
          loading={
            <div className="flex flex-col items-center gap-2 text-subtext-color">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-subtext-color"></div>
              <span>Loading PDF...</span>
            </div>
          }
          error={<div className="text-red-500">Failed to load PDF</div>}
          className="flex flex-col items-center gap-4"
        >
          {Array.from(new Array(numPages), (el, index) => (
            <Page
              key={`page_${index + 1}`}
              pageNumber={index + 1}
              renderTextLayer={false}
              renderAnnotationLayer={false}
              width={Math.min(window.innerWidth * 0.35, 500)} // Responsive width
              className="shadow-lg"
              canvasBackground="white"
            />
          ))}
        </Document>
      </div>
    </div>
  );
}
