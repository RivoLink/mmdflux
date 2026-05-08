export interface BrowserTextMetricsRequest {
  fontFamily: string;
  fontSizePx: number;
  lineHeightPx: number;
  fontStyle?: string;
  fontWeight?: string;
}

export interface PreparedBrowserTextMetrics {
  metricsJson: string;
  measureText: (text: string, cssFont: string) => number;
}

export interface BrowserTextMetricsEnvironment {
  OffscreenCanvas?: OffscreenCanvasFactory;
  fonts?: WorkerFontFaceSet;
}

interface OffscreenCanvasFactory {
  new (
    width: number,
    height: number,
  ): {
    getContext(type: "2d"): CanvasTextMeasureContext | null;
  };
}

interface CanvasTextMeasureContext {
  font: string;
  measureText(text: string): { width: number };
}

interface WorkerFontFaceSet {
  load(cssFont: string): Promise<unknown[]>;
  ready?: Promise<unknown>;
  check(cssFont: string): boolean;
}

export function browserTextMetricsEnvironment(
  scope: unknown = globalThis,
): BrowserTextMetricsEnvironment {
  const candidate = scope as Partial<BrowserTextMetricsEnvironment>;
  return {
    OffscreenCanvas: candidate.OffscreenCanvas,
    fonts: candidate.fonts,
  };
}

export async function prepareBrowserTextMetrics(
  input: BrowserTextMetricsRequest,
  environment = browserTextMetricsEnvironment(),
): Promise<PreparedBrowserTextMetrics> {
  const cssFont = buildCssFont(input);
  const fontSet = environment.fonts;
  if (!fontSet) {
    throw new Error("Dynamic text metrics require worker FontFaceSet support.");
  }

  const Canvas = environment.OffscreenCanvas;
  if (!Canvas) {
    throw new Error(
      "Dynamic text metrics require OffscreenCanvas in the worker.",
    );
  }

  const canvas = new Canvas(1, 1);
  const context = canvas.getContext("2d");
  if (!context) {
    throw new Error("Dynamic text metrics require a 2D canvas context.");
  }

  const loadedFaces = await fontSet.load(cssFont);
  if (fontSet.ready) {
    await fontSet.ready;
  }

  if (loadedFaces.length === 0 || !fontSet.check(cssFont)) {
    throw new Error(
      `Dynamic text metrics unavailable for font ${input.fontFamily}.`,
    );
  }

  const cache = new Map<string, number>();
  return {
    metricsJson: JSON.stringify({
      cssFont,
      fontFamily: normalizeFontFamily(input.fontFamily),
      fontSizePx: input.fontSizePx,
      lineHeightPx: input.lineHeightPx,
    }),
    measureText: (text: string, measuredCssFont: string): number => {
      const key = `${measuredCssFont}\0${text}`;
      const cached = cache.get(key);
      if (cached !== undefined) {
        return cached;
      }

      context.font = measuredCssFont;
      const width = context.measureText(text).width;
      if (!Number.isFinite(width) || width < 0) {
        throw new Error("Canvas measureText returned an invalid width.");
      }
      cache.set(key, width);
      return width;
    },
  };
}

export function buildCssFont(input: BrowserTextMetricsRequest): string {
  const fontFamily = normalizeFontFamily(input.fontFamily);
  validatePositiveFinite("fontSizePx", input.fontSizePx);
  validatePositiveFinite("lineHeightPx", input.lineHeightPx);

  const fontStyle = input.fontStyle?.trim() || "normal";
  const fontWeight = input.fontWeight?.trim() || "400";
  const quotedFamily = fontFamily.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
  return `${fontStyle} ${fontWeight} ${input.fontSizePx}px "${quotedFamily}"`;
}

function normalizeFontFamily(fontFamily: string): string {
  const normalized = fontFamily.trim();
  if (!normalized) {
    throw new Error("fontFamily must not be empty.");
  }
  return normalized;
}

function validatePositiveFinite(field: string, value: number): void {
  if (!Number.isFinite(value) || value <= 0) {
    throw new Error(`${field} must be a finite positive number.`);
  }
}
