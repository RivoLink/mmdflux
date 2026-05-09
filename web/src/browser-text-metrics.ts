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

export type BrowserTextMetricsCapabilityCode =
  | "worker-font-face-set-unavailable"
  | "worker-offscreen-canvas-unavailable"
  | "canvas-2d-context-unavailable"
  | "main-thread-font-face-set-unavailable"
  | "main-thread-canvas-unavailable"
  | "main-thread-canvas-2d-context-unavailable";

export class BrowserTextMetricsCapabilityError extends Error {
  readonly fallbackEligible: boolean;

  constructor(
    readonly code: BrowserTextMetricsCapabilityCode,
    message: string,
    fallbackEligible = true,
  ) {
    super(message);
    this.name = "BrowserTextMetricsCapabilityError";
    this.fallbackEligible = fallbackEligible;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

export function isBrowserTextMetricsCapabilityError(
  error: unknown,
): error is BrowserTextMetricsCapabilityError {
  return error instanceof BrowserTextMetricsCapabilityError;
}

export interface BrowserTextMetricsEnvironment {
  OffscreenCanvas?: OffscreenCanvasFactory;
  fonts?: BrowserFontFaceSet;
}

export interface MainThreadBrowserTextMetricsEnvironment {
  document?: MainThreadTextMetricsDocument;
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

interface BrowserFontFaceSet {
  load(cssFont: string): Promise<unknown[]>;
  ready?: Promise<unknown>;
  check(cssFont: string): boolean;
}

interface MainThreadTextMetricsDocument {
  fonts?: BrowserFontFaceSet;
  createElement?(tagName: "canvas"): {
    getContext(type: "2d"): CanvasTextMeasureContext | null;
  } | null;
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

export function mainThreadBrowserTextMetricsEnvironment(
  scope: unknown = globalThis,
): MainThreadBrowserTextMetricsEnvironment {
  const candidate = scope as Partial<MainThreadBrowserTextMetricsEnvironment>;
  return {
    document: candidate.document,
  };
}

export async function prepareBrowserTextMetrics(
  input: BrowserTextMetricsRequest,
  environment = browserTextMetricsEnvironment(),
): Promise<PreparedBrowserTextMetrics> {
  const cssFont = buildCssFont(input);
  const fontSet = environment.fonts;
  if (!fontSet) {
    throw new BrowserTextMetricsCapabilityError(
      "worker-font-face-set-unavailable",
      "Dynamic text metrics require worker FontFaceSet support.",
    );
  }

  const Canvas = environment.OffscreenCanvas;
  if (!Canvas) {
    throw new BrowserTextMetricsCapabilityError(
      "worker-offscreen-canvas-unavailable",
      "Dynamic text metrics require OffscreenCanvas in the worker.",
    );
  }

  const canvas = new Canvas(1, 1);
  const context = canvas.getContext("2d");
  if (!context) {
    throw new BrowserTextMetricsCapabilityError(
      "canvas-2d-context-unavailable",
      "Dynamic text metrics require a 2D canvas context.",
    );
  }

  await loadAndValidateFontSet(fontSet, cssFont, input.fontFamily);

  return preparedMetrics(input, cssFont, context);
}

export async function prepareMainThreadBrowserTextMetrics(
  input: BrowserTextMetricsRequest,
  environment = mainThreadBrowserTextMetricsEnvironment(),
): Promise<PreparedBrowserTextMetrics> {
  const cssFont = buildCssFont(input);
  const document = environment.document;
  if (!document?.fonts) {
    throw new BrowserTextMetricsCapabilityError(
      "main-thread-font-face-set-unavailable",
      "Dynamic text metrics require document.fonts on the main thread.",
      false,
    );
  }

  const fontSet = document.fonts;
  if (!document.createElement) {
    throw new BrowserTextMetricsCapabilityError(
      "main-thread-canvas-unavailable",
      "Dynamic text metrics require a main-thread canvas.",
      false,
    );
  }

  const canvas = document.createElement("canvas");
  if (!canvas) {
    throw new BrowserTextMetricsCapabilityError(
      "main-thread-canvas-unavailable",
      "Dynamic text metrics require a main-thread canvas.",
      false,
    );
  }

  const context = canvas.getContext("2d");
  if (!context) {
    throw new BrowserTextMetricsCapabilityError(
      "main-thread-canvas-2d-context-unavailable",
      "Dynamic text metrics require a main-thread 2D canvas context.",
      false,
    );
  }

  await loadAndValidateFontSet(fontSet, cssFont, input.fontFamily);

  return preparedMetrics(input, cssFont, context);
}

function preparedMetrics(
  input: BrowserTextMetricsRequest,
  cssFont: string,
  context: CanvasTextMeasureContext,
): PreparedBrowserTextMetrics {
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

async function loadAndValidateFontSet(
  fontSet: BrowserFontFaceSet,
  cssFont: string,
  fontFamily: string,
): Promise<void> {
  // Do not await FontFaceSet.ready here. Chrome worker FontFaceSet.ready can
  // stay pending for system-font stacks even after load resolves and check
  // passes; load plus post-load check is the requested-font contract.
  await fontSet.load(cssFont);
  if (!fontSet.check(cssFont)) {
    throw new Error(`Dynamic text metrics unavailable for font ${fontFamily}.`);
  }
}

export function buildCssFont(input: BrowserTextMetricsRequest): string {
  const fontFamily = fontFamilyStackToCss(input.fontFamily);
  validatePositiveFinite("fontSizePx", input.fontSizePx);
  validatePositiveFinite("lineHeightPx", input.lineHeightPx);

  const fontStyle = input.fontStyle?.trim() || "normal";
  const fontWeight = input.fontWeight?.trim() || "400";
  return `${fontStyle} ${fontWeight} ${input.fontSizePx}px ${fontFamily}`;
}

function normalizeFontFamily(fontFamily: string): string {
  const normalized = fontFamily.trim();
  if (!normalized) {
    throw new Error("fontFamily must not be empty.");
  }
  return normalized;
}

function fontFamilyStackToCss(fontFamily: string): string {
  return normalizeFontFamily(fontFamily)
    .split(",")
    .map((family) => familyTokenToCss(family))
    .join(", ");
}

function familyTokenToCss(family: string): string {
  const unquoted = stripOneQuoteLayer(family.trim());
  if (!unquoted) {
    throw new Error("fontFamily must not contain empty family names.");
  }

  if (isGenericFamily(unquoted)) {
    return unquoted.toLowerCase();
  }

  const escaped = unquoted.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
  return `"${escaped}"`;
}

function stripOneQuoteLayer(value: string): string {
  if (
    value.length >= 2 &&
    ((value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'")))
  ) {
    return value.slice(1, -1).trim();
  }

  return value;
}

function isGenericFamily(family: string): boolean {
  switch (family.toLowerCase()) {
    case "serif":
    case "sans-serif":
    case "monospace":
    case "cursive":
    case "fantasy":
    case "system-ui":
    case "ui-serif":
    case "ui-sans-serif":
    case "ui-monospace":
    case "ui-rounded":
    case "emoji":
    case "math":
    case "fangsong":
      return true;
    default:
      return false;
  }
}

function validatePositiveFinite(field: string, value: number): void {
  if (!Number.isFinite(value) || value <= 0) {
    throw new Error(`${field} must be a finite positive number.`);
  }
}
