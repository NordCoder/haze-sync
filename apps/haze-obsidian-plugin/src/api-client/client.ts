import { PluginSettings } from "../settings";

import {
  ApiClientError,
  createConfigurationError,
  createHttpError,
  createInvalidResponseError,
  createOfflineError,
} from "./errors";
import {
  ApiClientConfig,
  apiClientConfigFromSettings,
  assertConfiguredServer,
  assertSameOrigin,
  buildApiHeaders,
  buildApiUrl,
  buildFilePathUrl,
} from "./request";
import type {
  ChangesRequest,
  ChangesResponseDto,
  ConflictListRequest,
  ConflictsResponseDto,
  DeleteFileRequest,
  DeleteFileResponseDto,
  FileDownload,
  PutFileRequest,
  PutFileResponseDto,
  ResolveConflictRequest,
  ResolveConflictResponseDto,
  ServerInfoDto,
} from "./types";
import {
  isChangesResponseDto,
  isConflictsResponseDto,
  isDeleteFileResponseDto,
  isPutFileResponseDto,
  isResolveConflictResponseDto,
  isServerInfoDto,
} from "./validators";

export type HttpTransport = (url: string, init: RequestInit) => Promise<Response>;

export interface HazeSyncApiClientOptions {
  config: ApiClientConfig;
  transport?: HttpTransport;
  signal?: AbortSignal;
}

export class HazeSyncApiClient {
  private readonly config: ApiClientConfig;
  private readonly transport: HttpTransport;
  private readonly signal?: AbortSignal;

  constructor(options: HazeSyncApiClientOptions) {
    this.config = options.config;
    this.transport = options.transport ?? fetch;
    this.signal = options.signal;
  }

  static fromSettings(
    settings: PluginSettings,
    transport?: HttpTransport,
    signal?: AbortSignal,
  ): HazeSyncApiClient {
    return new HazeSyncApiClient({
      config: apiClientConfigFromSettings(settings),
      transport,
      signal,
    });
  }

  async getServerInfo(): Promise<ServerInfoDto> {
    return this.requestJson("GET", this.apiUrl("/v1/server-info"), {
      validate: isServerInfoDto,
    });
  }

  async getChanges(request: Partial<ChangesRequest> = {}): Promise<ChangesResponseDto> {
    return this.requestJson(
      "GET",
      this.apiUrl("/v1/changes", {
        since: request.since ?? 0,
        limit: request.limit ?? 50,
      }),
      { validate: isChangesResponseDto },
    );
  }

  async getFile(path: string): Promise<FileDownload> {
    const response = await this.requestRaw("GET", this.filePathUrl("/v1/files", path), {
      headers: buildApiHeaders(this.config.authToken, { accept: "application/octet-stream" }),
    });

    const sizeHeader = response.headers.get("X-Size-Bytes");
    const parsedSize = sizeHeader === null ? undefined : Number(sizeHeader);
    const metadata: FileDownload["metadata"] = {
      path,
      revision_id: response.headers.get("X-Revision-Id") ?? undefined,
      content_sha256: response.headers.get("X-Content-SHA256") ?? undefined,
      size_bytes:
        parsedSize !== undefined && Number.isSafeInteger(parsedSize) && parsedSize >= 0
          ? parsedSize
          : undefined,
    };

    return {
      metadata,
      body: await response.arrayBuffer(),
      contentType: response.headers.get("Content-Type"),
    };
  }

  async putFile(request: PutFileRequest): Promise<PutFileResponseDto> {
    return this.requestJson("PUT", this.filePathUrl("/v1/files", request.path), {
      body: request.body,
      headers: buildApiHeaders(this.config.authToken, {
        idempotencyKey: request.idempotencyKey,
        contentHash: request.contentHash,
        baseRevisionId: request.baseRevisionId,
        contentType: request.contentType ?? "application/octet-stream",
      }),
      validate: isPutFileResponseDto,
    });
  }

  async deleteFile(request: DeleteFileRequest): Promise<DeleteFileResponseDto> {
    return this.requestJson("DELETE", this.filePathUrl("/v1/files", request.path), {
      headers: buildApiHeaders(this.config.authToken, {
        idempotencyKey: request.idempotencyKey,
        baseRevisionId: request.baseRevisionId,
      }),
      validate: isDeleteFileResponseDto,
    });
  }

  async getConflicts(request: ConflictListRequest = {}): Promise<ConflictsResponseDto> {
    return this.requestJson(
      "GET",
      this.apiUrl("/v1/conflicts", { status: request.status }),
      { validate: isConflictsResponseDto },
    );
  }

  async resolveConflict(request: ResolveConflictRequest): Promise<ResolveConflictResponseDto> {
    return this.requestJson(
      "POST",
      this.apiUrl(`/v1/conflicts/${encodeURIComponent(request.conflictId)}/resolve`),
      {
        body: JSON.stringify({ resolution: request.resolution }),
        headers: buildApiHeaders(this.config.authToken, {
          idempotencyKey: request.idempotencyKey,
          contentType: "application/json",
        }),
        validate: isResolveConflictResponseDto,
      },
    );
  }

  private apiUrl(path: string, query?: Record<string, string | number | undefined>): URL {
    try {
      assertConfiguredServer(this.config);
      return buildApiUrl(this.config.serverUrl, path, query);
    } catch {
      throw createConfigurationError("Haze Sync Server URL is not configured correctly.");
    }
  }

  private filePathUrl(pathPrefix: string, path: string): URL {
    try {
      assertConfiguredServer(this.config);
      return buildFilePathUrl(this.config.serverUrl, pathPrefix, path);
    } catch {
      throw createConfigurationError("Haze Sync Server URL is not configured correctly.");
    }
  }

  private async requestJson<T>(method: string, url: URL, options: RequestJsonOptions<T>): Promise<T> {
    const response = await this.requestRaw(method, url, options);
    const payload = await readJsonPayload(response);

    if (!options.validate(payload)) {
      throw createInvalidResponseError(endpointForReport(url));
    }

    return payload;
  }

  private async requestRaw(method: string, url: URL, options: RequestRawOptions = {}): Promise<Response> {
    try {
      assertConfiguredServer(this.config);
      assertSameOrigin(url, this.config.serverUrl);
    } catch (error) {
      if (error instanceof ApiClientError) {
        throw error;
      }

      throw createConfigurationError(
        "Request URL does not match the configured Haze Sync Server origin.",
        endpointForReport(url),
      );
    }

    let response: Response;
    try {
      response = await this.transport(url.toString(), {
        method,
        body: options.body,
        headers: options.headers ?? buildApiHeaders(this.config.authToken),
        redirect: "error",
        credentials: "omit",
        cache: "no-store",
        signal: this.signal,
      });
    } catch (error) {
      if (error instanceof ApiClientError || isAbortError(error)) {
        throw error;
      }

      throw createOfflineError(endpointForReport(url));
    }

    if (!response.ok) {
      throw createHttpError(
        response.status,
        endpointForReport(url),
        await readJsonPayload(response),
        [this.config.authToken],
      );
    }

    return response;
  }
}

interface RequestRawOptions {
  body?: BodyInit;
  headers?: Headers;
}

interface RequestJsonOptions<T> extends RequestRawOptions {
  validate(payload: unknown): payload is T;
}

async function readJsonPayload(response: Response): Promise<unknown> {
  const contentType = response.headers.get("Content-Type") ?? "";
  if (!contentType.includes("application/json")) {
    return undefined;
  }

  try {
    return await response.json();
  } catch {
    return undefined;
  }
}

function endpointForReport(url: URL): string {
  return `${url.origin}${url.pathname}`;
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}
