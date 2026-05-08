import { describe, expect, it, vi } from "vitest";
import {
  type BrowserTextMetricsEnvironment,
  buildCssFont,
  prepareBrowserTextMetrics,
} from "./browser-text-metrics";

interface FakeTextMetrics {
  width: number;
}

interface FakeCanvasContext {
  font: string;
  measureText: (text: string) => FakeTextMetrics;
}

interface FakeFontFaceSet {
  load: (cssFont: string) => Promise<unknown[]>;
  ready: Promise<unknown>;
  check: (cssFont: string) => boolean;
}

function environmentFixture(
  fonts: FakeFontFaceSet | undefined = fontSetFixture(),
) {
  const context: FakeCanvasContext = {
    font: "",
    measureText: vi.fn((text: string) => ({ width: text.length * 3 })),
  };
  class FakeOffscreenCanvas {
    getContext(type: string): FakeCanvasContext | null {
      return type === "2d" ? context : null;
    }
  }

  const environment: BrowserTextMetricsEnvironment = {
    OffscreenCanvas: FakeOffscreenCanvas,
    fonts,
  };

  return {
    context,
    environment,
  };
}

function fontSetFixture(
  overrides: Partial<FakeFontFaceSet> = {},
): FakeFontFaceSet {
  return {
    load: vi.fn(async () => [{}]),
    ready: Promise.resolve(),
    check: vi.fn(() => true),
    ...overrides,
  };
}

describe("prepareBrowserTextMetrics", () => {
  it("fails honestly without OffscreenCanvas", async () => {
    await expect(
      prepareBrowserTextMetrics(
        { fontFamily: "Inter", fontSizePx: 16, lineHeightPx: 24 },
        { fonts: fontSetFixture() },
      ),
    ).rejects.toThrow("OffscreenCanvas");
  });

  it("fails honestly without worker FontFaceSet support", async () => {
    const { environment } = environmentFixture();
    environment.fonts = undefined;

    await expect(
      prepareBrowserTextMetrics(
        { fontFamily: "Inter", fontSizePx: 16, lineHeightPx: 24 },
        environment,
      ),
    ).rejects.toThrow("FontFaceSet");
  });

  it("uses load for readiness and post-load check for validity", async () => {
    const calls: string[] = [];
    const fonts = fontSetFixture({
      load: vi.fn(async () => {
        calls.push("load");
        return [{}];
      }),
      ready: Promise.resolve().then(() => {
        calls.push("ready");
      }),
      check: vi.fn(() => {
        calls.push("check");
        return true;
      }),
    });
    const { environment } = environmentFixture(fonts);

    const prepared = await prepareBrowserTextMetrics(
      { fontFamily: "Inter", fontSizePx: 16, lineHeightPx: 24 },
      environment,
    );

    expect(fonts.load).toHaveBeenCalledWith('normal 400 16px "Inter"');
    expect(fonts.check).toHaveBeenCalledWith('normal 400 16px "Inter"');
    expect(calls).toEqual(["load", "ready", "check"]);
    expect(JSON.parse(prepared.metricsJson)).toEqual({
      cssFont: 'normal 400 16px "Inter"',
      fontFamily: "Inter",
      fontSizePx: 16,
      lineHeightPx: 24,
    });
  });

  it("rejects zero loaded faces as unavailable font", async () => {
    const { environment } = environmentFixture(
      fontSetFixture({
        load: vi.fn(async () => []),
      }),
    );

    await expect(
      prepareBrowserTextMetrics(
        { fontFamily: "Inter", fontSizePx: 16, lineHeightPx: 24 },
        environment,
      ),
    ).rejects.toThrow("unavailable");
  });

  it("returns a synchronous finite width from canvas measureText", async () => {
    const { context, environment } = environmentFixture();
    const prepared = await prepareBrowserTextMetrics(
      { fontFamily: "Inter", fontSizePx: 16, lineHeightPx: 24 },
      environment,
    );

    expect(prepared.measureText("Alpha", 'normal 400 16px "Inter"')).toBe(15);
    expect(context.font).toBe('normal 400 16px "Inter"');
  });

  it("caches repeated exact text and font measurements", async () => {
    const { context, environment } = environmentFixture();
    const prepared = await prepareBrowserTextMetrics(
      { fontFamily: "Inter", fontSizePx: 16, lineHeightPx: 24 },
      environment,
    );

    prepared.measureText("Alpha", 'normal 400 16px "Inter"');
    prepared.measureText("Alpha", 'normal 400 16px "Inter"');
    prepared.measureText("Alpha", 'normal 400 18px "Inter"');

    expect(context.measureText).toHaveBeenCalledTimes(2);
  });

  it("quotes CSS font families and rejects invalid numeric style fields", async () => {
    expect(
      buildCssFont({
        fontFamily: "Open Sans",
        fontSizePx: 16,
        lineHeightPx: 24,
      }),
    ).toBe('normal 400 16px "Open Sans"');

    await expect(
      prepareBrowserTextMetrics(
        { fontFamily: "Inter", fontSizePx: 0, lineHeightPx: 24 },
        environmentFixture().environment,
      ),
    ).rejects.toThrow("fontSizePx");

    await expect(
      prepareBrowserTextMetrics(
        { fontFamily: "Inter", fontSizePx: 16, lineHeightPx: Number.NaN },
        environmentFixture().environment,
      ),
    ).rejects.toThrow("lineHeightPx");
  });
});
