import { prepareMainThreadBrowserTextMetrics } from "../browser-text-metrics";
import { loadWasmModule, type WasmModule } from "../wasm-module";
import type {
  BrowserTextMetricsRenderRequest,
  RenderResponse,
} from "./render-client";

export interface MainThreadBrowserTextMetricsRenderer {
  renderWithBrowserTextMetrics: (
    request: BrowserTextMetricsRenderRequest,
  ) => Promise<RenderResponse>;
}

export interface MainThreadBrowserTextMetricsRendererOptions {
  loadWasmModule?: () => Promise<WasmModule>;
  prepareMainThreadBrowserTextMetrics?: typeof prepareMainThreadBrowserTextMetrics;
}

export function createMainThreadBrowserTextMetricsRenderer(
  options: MainThreadBrowserTextMetricsRendererOptions = {},
): MainThreadBrowserTextMetricsRenderer {
  const loadModule = options.loadWasmModule ?? loadWasmModule;
  const prepareMetrics =
    options.prepareMainThreadBrowserTextMetrics ??
    prepareMainThreadBrowserTextMetrics;
  let modulePromise: Promise<WasmModule> | null = null;

  const getWasmModule = async (): Promise<WasmModule> => {
    if (!modulePromise) {
      modulePromise = loadModule().then(async (module) => {
        await module.default();
        return module;
      });
    }

    return modulePromise;
  };

  return {
    renderWithBrowserTextMetrics: async (request) => {
      const prepared = await prepareMetrics(request.browserTextMetrics);
      const wasmModule = await getWasmModule();
      const output = wasmModule.renderWithBrowserTextMetrics(
        request.input,
        "svg",
        request.configJson ?? "{}",
        prepared.metricsJson,
        prepared.measureText,
      );

      return {
        seq: request.seq,
        format: "svg",
        output,
      };
    },
  };
}
