'use client';

import { useCallback, useEffect, useRef, useState } from 'react';

interface PdfPreviewProps {
  pdfBytes: Uint8Array | null;
  className?: string;
}

/**
 * Renders PDF bytes to canvas elements using pdf.js.
 *
 * Shows all pages stacked vertically with a subtle gap between them,
 * scaled to fit the container width.
 */
export function PdfPreview({ pdfBytes, className = '' }: PdfPreviewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [pageCanvases, setPageCanvases] = useState<HTMLCanvasElement[]>([]);
  const [error, setError] = useState<string | null>(null);

  const renderPdf = useCallback(async (bytes: Uint8Array) => {
    try {
      const pdfjsLib = await import('pdfjs-dist');

      // Use the bundled worker
      pdfjsLib.GlobalWorkerOptions.workerSrc = new URL(
        'pdfjs-dist/build/pdf.worker.min.mjs',
        import.meta.url
      ).toString();

      const doc = await pdfjsLib.getDocument({ data: bytes }).promise;
      const canvases: HTMLCanvasElement[] = [];

      const containerWidth = containerRef.current?.clientWidth ?? 600;
      // Leave some padding
      const targetWidth = containerWidth - 32;

      for (let i = 1; i <= doc.numPages; i++) {
        const page = await doc.getPage(i);
        const viewport = page.getViewport({ scale: 1 });

        // Scale to fit container width
        const scale = targetWidth / viewport.width;
        const scaledViewport = page.getViewport({ scale });

        const canvas = document.createElement('canvas');
        canvas.width = scaledViewport.width * 2; // 2x for retina
        canvas.height = scaledViewport.height * 2;
        canvas.style.width = `${scaledViewport.width}px`;
        canvas.style.height = `${scaledViewport.height}px`;

        const ctx = canvas.getContext('2d');
        if (ctx) {
          ctx.scale(2, 2);
          await page.render({
            canvasContext: ctx,
            viewport: scaledViewport,
          }).promise;
        }

        canvases.push(canvas);
      }

      setPageCanvases(canvases);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to render PDF');
      setPageCanvases([]);
    }
  }, []);

  useEffect(() => {
    if (pdfBytes && pdfBytes.length > 0) {
      renderPdf(pdfBytes);
    } else {
      setPageCanvases([]);
    }
  }, [pdfBytes, renderPdf]);

  if (error) {
    return (
      <div className={`flex items-center justify-center p-8 ${className}`}>
        <div className="text-center">
          <div className="mb-2 text-sc-error">Render Error</div>
          <div className="max-w-sm text-xs text-sc-fg-dim">{error}</div>
        </div>
      </div>
    );
  }

  if (pageCanvases.length === 0) {
    return null;
  }

  return (
    <div ref={containerRef} className={`flex flex-col items-center gap-4 ${className}`}>
      {pageCanvases.map((canvas, i) => (
        <div
          key={i}
          className="overflow-hidden rounded shadow-xl"
          ref={el => {
            if (el && !el.contains(canvas)) {
              el.innerHTML = '';
              el.appendChild(canvas);
            }
          }}
        />
      ))}
    </div>
  );
}
