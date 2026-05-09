import { readFile } from "node:fs/promises";
import path from "node:path";
import { describe, expect, it, vi } from "vitest";
import { createDefaultRenderWorkerClient, renderApp } from "./main";
import type { BrowserTextMetricsRenderRequest } from "./services/render-client";

describe("renderApp", () => {
  it("main bootstraps the app without owning render or persistence logic", async () => {
    const source = await readFile(
      path.resolve(process.cwd(), "src/main.ts"),
      "utf8",
    );

    expect(source).toMatch(/bootstrapPlaygroundApp/);
    expect(source).not.toMatch(/localStorage\.setItem/);
    expect(source).not.toMatch(/new Worker/);
  });

  it("wires main-thread browser metrics fallback into the default worker client", () => {
    const client = {
      render: vi.fn(),
      renderWithBrowserTextMetrics: vi.fn(),
      validate: vi.fn(),
      terminate: vi.fn(),
    };
    const fallbackRenderer = {
      renderWithBrowserTextMetrics: vi.fn(),
    };
    const createClient = vi.fn(() => client);
    const createFallbackRenderer = vi.fn(() => fallbackRenderer);

    vi.stubGlobal("Worker", class FakeWorker {});
    try {
      expect(
        createDefaultRenderWorkerClient(createClient, createFallbackRenderer),
      ).toBe(client);
    } finally {
      vi.unstubAllGlobals();
    }

    expect(createFallbackRenderer).toHaveBeenCalledTimes(1);
    expect(createClient).toHaveBeenCalledWith(undefined, {
      mainThreadBrowserTextMetricsRenderer: fallbackRenderer,
    });
  });

  it("renders redesigned playground shell", () => {
    try {
      history.replaceState(null, "", window.location.pathname);

      const root = document.createElement("div");
      renderApp(root, {
        renderClientFactory: () => ({
          render: async (request) => ({
            seq: request.seq,
            format: request.format,
            output: `${request.format}:${request.input}`,
          }),
          renderWithBrowserTextMetrics: async (request) => ({
            seq: request.seq,
            format: "svg",
            output: `svg:${request.input}`,
          }),
          validate: async () => '{"valid":true}',
          terminate: () => {},
        }),
        stateStorage: {
          getItem: () => null,
          setItem: () => {},
        },
      });
      const exampleSelect = root.querySelector<HTMLSelectElement>(
        "[data-example-select]",
      );
      const activeFormat = root.querySelector<HTMLButtonElement>(
        ".format-tabs button.is-active",
      );

      expect(root.textContent).toContain("mmdflux playground");
      expect(root.textContent).toContain("Advanced controls");
      expect(root.textContent).toContain("Syntax snippets");
      expect(activeFormat?.dataset.format).toBe("svg");
      expect(root.querySelector("[data-preview-controls]")).not.toBeNull();
      expect(root.querySelector("[data-theme-toggle]")).not.toBeNull();
      expect(exampleSelect?.value).toBe("__draft__");
      expect(window.__mmdfluxDebug).toBeUndefined();
    } finally {
      history.replaceState(null, "", window.location.pathname);
      delete window.__mmdfluxDebug;
    }
  });

  it("installs a query-gated browser metrics debug console helper", async () => {
    const renderWithBrowserTextMetrics = vi.fn(
      async (request: BrowserTextMetricsRenderRequest) => ({
        seq: request.seq,
        format: "svg" as const,
        output: `<span>${request.browserTextMetrics.fontFamily}:${request.input}</span>`,
      }),
    );
    const mainThreadRenderWithBrowserTextMetrics = vi.fn(
      async (request: BrowserTextMetricsRenderRequest) => ({
        seq: request.seq,
        format: "svg" as const,
        output: `<span>main-thread:${request.input}</span>`,
      }),
    );

    try {
      history.replaceState(null, "", "?debugBrowserMetrics=1");

      const root = document.createElement("div");
      renderApp(root, {
        renderClientFactory: () => ({
          render: async (request) => ({
            seq: request.seq,
            format: request.format,
            output: `${request.format}:${request.input}`,
          }),
          renderWithBrowserTextMetrics,
          validate: async () => '{"valid":true}',
          terminate: () => {},
        }),
        mainThreadBrowserTextMetricsRendererFactory: () => ({
          renderWithBrowserTextMetrics: mainThreadRenderWithBrowserTextMetrics,
        }),
        debounceMs: 10_000,
        stateStorage: {
          getItem: () => null,
          setItem: () => {},
        },
      });

      const debug = window.__mmdfluxDebug;
      expect(debug).toBeDefined();

      const workerResult = await debug?.renderBrowserMetrics({
        input: "graph TD\nA-->B",
        fontFamily: "Arial",
        fontSizePx: 18,
        lineHeightPx: 27,
      });

      expect(workerResult).toMatchObject({
        format: "svg",
        output: "<span>Arial:graph TD\nA-->B</span>",
        source: "worker-client",
      });
      expect(renderWithBrowserTextMetrics).toHaveBeenCalledWith({
        seq: expect.any(Number),
        input: "graph TD\nA-->B",
        configJson: '{"pathSimplification":"lossless"}',
        browserTextMetrics: {
          fontFamily: "Arial",
          fontSizePx: 18,
          lineHeightPx: 27,
        },
      });
      expect(root.querySelector("[data-preview-output]")?.textContent).toBe(
        "Arial:graph TD\nA-->B",
      );

      await debug?.renderBrowserMetricsMainThread({
        input: "graph TD\nM-->N",
        show: false,
      });

      expect(mainThreadRenderWithBrowserTextMetrics).toHaveBeenCalledWith({
        seq: expect.any(Number),
        input: "graph TD\nM-->N",
        configJson: '{"pathSimplification":"lossless"}',
        browserTextMetrics: {
          fontFamily: "Arial, sans-serif",
          fontSizePx: 16,
          lineHeightPx: 24,
        },
      });
      expect(renderWithBrowserTextMetrics).toHaveBeenCalledTimes(1);
    } finally {
      history.replaceState(null, "", window.location.pathname);
      delete window.__mmdfluxDebug;
    }
  });
});
