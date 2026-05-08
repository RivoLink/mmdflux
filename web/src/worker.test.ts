import { describe, expect, it, vi } from "vitest";
import { createWorkerRequestHandler } from "./worker";
import type {
  WorkerRequestMessage,
  WorkerResponseMessage,
} from "./worker-protocol";

interface MockWasmModule {
  default: () => Promise<void>;
  render: (input: string, format: string, configJson: string) => string;
  renderWithBrowserTextMetrics: (
    input: string,
    format: string,
    configJson: string,
    metricsJson: string,
    measureText: (text: string, cssFont: string) => number,
  ) => string;
  validate: (input: string) => string;
}

describe("createWorkerRequestHandler", () => {
  it("initializes worker once and returns render results", async () => {
    const initialize = vi.fn(async () => {});
    const render = vi.fn(
      (input: string, format: string, configJson: string) => {
        return `${format}:${input}:${configJson}`;
      },
    );
    const validate = vi.fn(
      (input: string) => `{"valid":true,"input":"${input}"}`,
    );
    const loadWasmModule = vi.fn(
      async (): Promise<MockWasmModule> => ({
        default: initialize,
        render,
        renderWithBrowserTextMetrics: () => "unused",
        validate,
      }),
    );

    const responses: WorkerResponseMessage[] = [];
    const handler = createWorkerRequestHandler({
      loadWasmModule,
      postMessage: (message) => responses.push(message),
    });

    const first: WorkerRequestMessage = {
      type: "render",
      seq: 1,
      input: "graph TD\nA-->B",
      format: "text",
      configJson: "{}",
    };
    const second: WorkerRequestMessage = {
      type: "render",
      seq: 2,
      input: "graph TD\nB-->C",
      format: "svg",
      configJson: '{"padding":2}',
    };

    await handler(first);
    await handler(second);

    expect(loadWasmModule).toHaveBeenCalledTimes(1);
    expect(initialize).toHaveBeenCalledTimes(1);
    expect(render).toHaveBeenCalledTimes(2);
    expect(validate).not.toHaveBeenCalled();
    expect(responses).toEqual([
      {
        type: "result",
        seq: 1,
        format: "text",
        output: "text:graph TD\nA-->B:{}",
      },
      {
        type: "result",
        seq: 2,
        format: "svg",
        output: 'svg:graph TD\nB-->C:{"padding":2}',
      },
    ]);
  });

  it("returns structured error payload on render failure", async () => {
    const loadWasmModule = vi.fn(
      async (): Promise<MockWasmModule> => ({
        default: async () => {},
        render: () => {
          throw new Error("unknown output format: bad");
        },
        renderWithBrowserTextMetrics: () => "unused",
        validate: () => '{"valid":true}',
      }),
    );

    const responses: WorkerResponseMessage[] = [];
    const handler = createWorkerRequestHandler({
      loadWasmModule,
      postMessage: (message) => responses.push(message),
    });

    const request: WorkerRequestMessage = {
      type: "render",
      seq: 42,
      input: "graph TD\nA-->B",
      format: "text",
      configJson: "{}",
    };

    await handler(request);

    expect(responses).toHaveLength(1);
    expect(responses[0]).toEqual({
      type: "error",
      seq: 42,
      error: "unknown output format: bad",
    });
  });

  it("returns validation results without reinitializing wasm", async () => {
    const initialize = vi.fn(async () => {});
    const loadWasmModule = vi.fn(
      async (): Promise<MockWasmModule> => ({
        default: initialize,
        render: () => "unused",
        renderWithBrowserTextMetrics: () => "unused",
        validate: (input) =>
          JSON.stringify({
            valid: input.includes("A-->B"),
            diagnostics: input.includes("A-->B")
              ? []
              : [{ message: "expected edge" }],
          }),
      }),
    );

    const responses: WorkerResponseMessage[] = [];
    const handler = createWorkerRequestHandler({
      loadWasmModule,
      postMessage: (message) => responses.push(message),
    });

    const request: WorkerRequestMessage = {
      type: "validate",
      seq: -1,
      input: "graph TD\nA-->B",
    };

    await handler(request);

    expect(loadWasmModule).toHaveBeenCalledTimes(1);
    expect(initialize).toHaveBeenCalledTimes(1);
    expect(responses).toEqual([
      {
        type: "validation",
        seq: -1,
        resultJson: '{"valid":true,"diagnostics":[]}',
      },
    ]);
  });

  it("prepares browser metrics and invokes the dynamic wasm export", async () => {
    const measureText = vi.fn(() => 42);
    const prepareBrowserTextMetrics = vi.fn(async () => ({
      metricsJson: '{"cssFont":"16px Inter"}',
      measureText,
    }));
    const renderWithBrowserTextMetrics = vi.fn(
      (
        input: string,
        format: string,
        configJson: string,
        metricsJson: string,
        callback: (text: string, cssFont: string) => number,
      ) =>
        `${format}:${input}:${configJson}:${metricsJson}:${callback("A", "font")}`,
    );
    const loadWasmModule = vi.fn(
      async (): Promise<MockWasmModule> => ({
        default: async () => {},
        render: () => "static unused",
        renderWithBrowserTextMetrics,
        validate: () => '{"valid":true}',
      }),
    );

    const responses: WorkerResponseMessage[] = [];
    const handler = createWorkerRequestHandler({
      loadWasmModule,
      prepareBrowserTextMetrics,
      postMessage: (message) => responses.push(message),
    });

    await handler({
      type: "renderWithBrowserTextMetrics",
      seq: 9,
      input: "graph TD\nA-->B",
      format: "svg",
      configJson: "{}",
      browserTextMetrics: {
        fontFamily: "Inter",
        fontSizePx: 16,
        lineHeightPx: 24,
      },
    });

    expect(prepareBrowserTextMetrics).toHaveBeenCalledWith({
      fontFamily: "Inter",
      fontSizePx: 16,
      lineHeightPx: 24,
    });
    expect(renderWithBrowserTextMetrics).toHaveBeenCalledWith(
      "graph TD\nA-->B",
      "svg",
      "{}",
      '{"cssFont":"16px Inter"}',
      measureText,
    );
    expect(responses).toEqual([
      {
        type: "result",
        seq: 9,
        format: "svg",
        output: 'svg:graph TD\nA-->B:{}:{"cssFont":"16px Inter"}:42',
      },
    ]);
  });

  it("returns structured errors for dynamic metric preparation failures", async () => {
    const loadWasmModule = vi.fn(
      async (): Promise<MockWasmModule> => ({
        default: async () => {},
        render: () => "static unused",
        renderWithBrowserTextMetrics: () => "dynamic unused",
        validate: () => '{"valid":true}',
      }),
    );
    const responses: WorkerResponseMessage[] = [];
    const handler = createWorkerRequestHandler({
      loadWasmModule,
      prepareBrowserTextMetrics: vi.fn(async () => {
        throw new Error("Dynamic text metrics require OffscreenCanvas");
      }),
      postMessage: (message) => responses.push(message),
    });

    await handler({
      type: "renderWithBrowserTextMetrics",
      seq: 10,
      input: "graph TD\nA-->B",
      format: "svg",
      configJson: "{}",
      browserTextMetrics: {
        fontFamily: "Inter",
        fontSizePx: 16,
        lineHeightPx: 24,
      },
    });

    expect(responses).toEqual([
      {
        type: "error",
        seq: 10,
        error: "Dynamic text metrics require OffscreenCanvas",
      },
    ]);
  });
});
