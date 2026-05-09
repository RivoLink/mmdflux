import { describe, expect, it, vi } from "vitest";
import type { MainThreadBrowserTextMetricsRenderer } from "./services/main-thread-browser-text-metrics";
import { createRenderWorkerClient } from "./services/render-client";
import type {
  WorkerRequestMessage,
  WorkerResponseMessage,
} from "./worker-protocol";

interface MockWorkerOptions {
  dynamicResponse?: WorkerResponseMessage;
  suppressDynamicResponse?: boolean;
  throwOnDynamicPost?: boolean;
}

class MockWorker {
  onmessage: ((event: MessageEvent<WorkerResponseMessage>) => void) | null =
    null;
  messages: WorkerRequestMessage[] = [];

  constructor(private readonly options: MockWorkerOptions = {}) {}

  postMessage(message: WorkerRequestMessage): void {
    this.messages.push(message);
    if (
      message.type === "renderWithBrowserTextMetrics" &&
      this.options.throwOnDynamicPost
    ) {
      throw new Error("worker post failed");
    }

    if (!this.onmessage) {
      throw new Error("worker message handler was not installed");
    }

    queueMicrotask(() => {
      if (message.type === "render") {
        this.onmessage?.({
          data: {
            type: "result",
            seq: message.seq,
            format: message.format,
            output: `${message.format}:${message.input}:${message.configJson}`,
          },
        } as MessageEvent<WorkerResponseMessage>);
        return;
      }

      if (message.type === "renderWithBrowserTextMetrics") {
        if (this.options.suppressDynamicResponse) {
          return;
        }

        if (this.options.dynamicResponse) {
          this.onmessage?.({
            data: this.options.dynamicResponse,
          } as MessageEvent<WorkerResponseMessage>);
          return;
        }

        this.onmessage?.({
          data: {
            type: "result",
            seq: message.seq,
            format: message.format,
            output: `${message.format}:${message.input}:${message.configJson}:${message.browserTextMetrics.fontFamily}`,
          },
        } as MessageEvent<WorkerResponseMessage>);
        return;
      }

      this.onmessage?.({
        data: {
          type: "validation",
          seq: message.seq,
          resultJson: '{"valid":true}',
        },
      } as MessageEvent<WorkerResponseMessage>);
    });
  }

  terminate(): void {}
}

function mainThreadRendererFixture(
  output = "main-thread-svg",
): MainThreadBrowserTextMetricsRenderer {
  return {
    renderWithBrowserTextMetrics: vi.fn(async (request) => ({
      seq: request.seq,
      format: "svg" as const,
      output,
    })),
  };
}

describe("createRenderWorkerClient", () => {
  it("routes render and validation requests over the same worker", async () => {
    const worker = new MockWorker();
    const client = createRenderWorkerClient(worker as unknown as Worker);

    const renderPromise = client.render({
      seq: 7,
      input: "graph TD\nA-->B",
      format: "svg",
      configJson: '{"padding":2}',
    });
    const validatePromise = client.validate("graph TD\nA-->B");

    await expect(renderPromise).resolves.toEqual({
      seq: 7,
      format: "svg",
      output: 'svg:graph TD\nA-->B:{"padding":2}',
    });
    await expect(validatePromise).resolves.toBe('{"valid":true}');
  });

  it("posts dynamic browser text metrics render requests separately", async () => {
    const worker = new MockWorker();
    const mainThreadRenderer = mainThreadRendererFixture();
    const client = createRenderWorkerClient(worker as unknown as Worker, {
      mainThreadBrowserTextMetricsRenderer: mainThreadRenderer,
    });

    const response = await client.renderWithBrowserTextMetrics({
      seq: 11,
      input: "graph TD\nA-->B",
      configJson: "{}",
      browserTextMetrics: {
        fontFamily: "Inter",
        fontSizePx: 16,
        lineHeightPx: 24,
      },
    });

    expect(
      mainThreadRenderer.renderWithBrowserTextMetrics,
    ).not.toHaveBeenCalled();
    expect(response).toEqual({
      seq: 11,
      format: "svg",
      output: "svg:graph TD\nA-->B:{}:Inter",
    });
    expect(worker.messages.at(-1)).toEqual({
      type: "renderWithBrowserTextMetrics",
      seq: 11,
      input: "graph TD\nA-->B",
      format: "svg",
      configJson: "{}",
      browserTextMetrics: {
        fontFamily: "Inter",
        fontSizePx: 16,
        lineHeightPx: 24,
      },
    });
  });

  it("falls back to main-thread dynamic rendering on worker capability errors", async () => {
    const worker = new MockWorker({
      dynamicResponse: {
        type: "error",
        seq: 11,
        error: "Dynamic text metrics require OffscreenCanvas in the worker.",
        code: "dynamic-metrics-capability",
      },
    });
    const mainThreadRenderer = mainThreadRendererFixture();
    const client = createRenderWorkerClient(worker as unknown as Worker, {
      mainThreadBrowserTextMetricsRenderer: mainThreadRenderer,
    });

    await expect(
      client.renderWithBrowserTextMetrics({
        seq: 11,
        input: "graph TD\nA-->B",
        configJson: "{}",
        browserTextMetrics: {
          fontFamily: "Inter",
          fontSizePx: 16,
          lineHeightPx: 24,
        },
      }),
    ).resolves.toEqual({
      seq: 11,
      format: "svg",
      output: "main-thread-svg",
    });
    expect(
      mainThreadRenderer.renderWithBrowserTextMetrics,
    ).toHaveBeenCalledTimes(1);
  });

  it("falls back to main-thread dynamic rendering when the worker does not respond", async () => {
    const worker = new MockWorker({ suppressDynamicResponse: true });
    const mainThreadRenderer = mainThreadRendererFixture();
    const client = createRenderWorkerClient(worker as unknown as Worker, {
      mainThreadBrowserTextMetricsRenderer: mainThreadRenderer,
      dynamicMetricsWorkerTimeoutMs: 1,
    });

    await expect(
      client.renderWithBrowserTextMetrics({
        seq: 14,
        input: "graph TD\nA-->B",
        configJson: "{}",
        browserTextMetrics: {
          fontFamily: "Arial",
          fontSizePx: 16,
          lineHeightPx: 24,
        },
      }),
    ).resolves.toEqual({
      seq: 14,
      format: "svg",
      output: "main-thread-svg",
    });
    expect(
      mainThreadRenderer.renderWithBrowserTextMetrics,
    ).toHaveBeenCalledTimes(1);
  });

  it("does not fallback on ordinary dynamic worker errors", async () => {
    const worker = new MockWorker({
      dynamicResponse: {
        type: "error",
        seq: 12,
        error: "Dynamic text metrics unavailable for font Inter.",
      },
    });
    const mainThreadRenderer = mainThreadRendererFixture();
    const client = createRenderWorkerClient(worker as unknown as Worker, {
      mainThreadBrowserTextMetricsRenderer: mainThreadRenderer,
    });

    await expect(
      client.renderWithBrowserTextMetrics({
        seq: 12,
        input: "graph TD\nA-->B",
        configJson: "{}",
        browserTextMetrics: {
          fontFamily: "Inter",
          fontSizePx: 16,
          lineHeightPx: 24,
        },
      }),
    ).rejects.toThrow("unavailable");
    expect(
      mainThreadRenderer.renderWithBrowserTextMetrics,
    ).not.toHaveBeenCalled();
  });

  it("does not fallback when posting dynamic requests fails", async () => {
    const worker = new MockWorker({ throwOnDynamicPost: true });
    const mainThreadRenderer = mainThreadRendererFixture();
    const client = createRenderWorkerClient(worker as unknown as Worker, {
      mainThreadBrowserTextMetricsRenderer: mainThreadRenderer,
    });

    await expect(
      client.renderWithBrowserTextMetrics({
        seq: 13,
        input: "graph TD\nA-->B",
        configJson: "{}",
        browserTextMetrics: {
          fontFamily: "Inter",
          fontSizePx: 16,
          lineHeightPx: 24,
        },
      }),
    ).rejects.toThrow("failed to post dynamic render request");
    expect(
      mainThreadRenderer.renderWithBrowserTextMetrics,
    ).not.toHaveBeenCalled();
  });

  it("does not use main-thread dynamic rendering for validation", async () => {
    const worker = new MockWorker();
    const mainThreadRenderer = mainThreadRendererFixture();
    const client = createRenderWorkerClient(worker as unknown as Worker, {
      mainThreadBrowserTextMetricsRenderer: mainThreadRenderer,
    });

    await expect(client.validate("graph TD\nA-->B")).resolves.toBe(
      '{"valid":true}',
    );
    expect(
      mainThreadRenderer.renderWithBrowserTextMetrics,
    ).not.toHaveBeenCalled();
  });
});
